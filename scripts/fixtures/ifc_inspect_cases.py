#!/usr/bin/env python3
"""Shared cases of `cad.ifc.inspect` (plan/ifc-uyum.md IFC-12, docs/adr/8073 §6), written from another reader's reading
of the same files: IfcOpenShell (not KentOS code, not KentOS's reading engine) gives each file's schema, records,
objects by class, project units, storeys with what each contains directly, and the asked object's attributes and its
property and quantity sets as the file writes them (its own in the order of their relations' records, then its type's
in its HasPropertySets order). The broken files are made here from a sound one, so what is wrong in each is known by
how it was made: the record and line of a reference to nothing and of a GlobalId said twice.

Writes fixtures/ifc/v1/bozuk/*.ifc and fixtures/commands/v1/cad.ifc.inspect.json; --check compares both, writing
nothing. Runs under IfcOpenShell, offline:

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 python -I scripts/fixtures/ifc_inspect_cases.py [--check]
"""

import json
import re
import sys
from collections import Counter
from pathlib import Path

import ifcopenshell
import ifcopenshell.util.unit as unit

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.ifc.inspect.json"
BROKEN = ROOT / "fixtures" / "ifc" / "v1" / "bozuk"
# Reading a file changes nothing in the drawing.
SAME = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "IFC incelemesi",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                 "workspace": "cad"},
    "origin": {"x": 0, "y": 0},
    "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
    "activeLayer": "cizim",
    "styles": {"items": [], "categories": []},
    "entities": [],
}
# The files read whole: KentOS's, another program's, then the real-world ones (fixtures/ifc/v1/disaridan/README.md).
FILES = [
    ("Kesit binası (KentOS)", "arch/v1/ifc/kesit-binasi.ifc"),
    ("Kesit binası künyeli (KentOS)", "arch/v1/ifc/kesit-binasi-kunye.ifc"),
    ("İki bloklu proje, A bloğu (KentOS)", "arch/v1/ifc/blok-a.ifc"),
    ("İki bloklu proje, B bloğu (KentOS)", "arch/v1/ifc/blok-b.ifc"),
    ("Çatı altında mahal (KentOS)", "arch/v1/ifc/cati-alti-mahal.ifc"),
    ("Eğimli döşeme (KentOS)", "arch/v1/ifc/egimli-doseme.ifc"),
    ("Eğri bina (KentOS)", "arch/v1/ifc/egri-bina.ifc"),
    ("Eğri bina, kesilmiş gövdeler (KentOS)", "arch/v1/ifc/egri-bina-referans.ifc"),
    ("Katmanlı çatı (KentOS)", "arch/v1/ifc/katmanli-cati.ifc"),
    ("Yerel sistemde bina (KentOS)", "arch/v1/ifc/yerel-sistem.ifc"),
    ("Yolun ekseni ve boykesiti (KentOS)", "road/v1/ifc/out/yol.ifc"),
    ("Başka bir programın IFC4 dosyası", "ifc/v1/acik/ifcopenshell-ifc4.ifc"),
    ("Başka bir programın IFC 4.3 dosyası", "ifc/v1/acik/ifcopenshell-ifc4x3.ifc"),
    ("Gerçek örnek: IFC 4.3 bina", "ifc/v1/disaridan/bsi-ifc4x3-building-architecture.ifc"),
    ("Gerçek örnek: IFC 4.3 yol", "ifc/v1/disaridan/bsi-ifc4x3-infra-road.ifc"),
    ("Gerçek örnek: IFC 4.3 demiryolu", "ifc/v1/disaridan/bsi-ifc4x3-infra-rail.ifc"),
    ("Gerçek örnek: IFC4 açıklıklı duvar ve pencere", "ifc/v1/disaridan/bsi-ifc4-wall-with-opening-and-window.ifc"),
    ("Gerçek örnek: IFC4 lavabo", "ifc/v1/disaridan/bsi-ifc4-basin-tessellation.ifc"),
    ("Gerçek örnek: IFC4 kolon", "ifc/v1/disaridan/bsi-ifc4-column-straight-rectangle-tessellation.ifc"),
    ("Gerçek örnek: IFC2x3 bina", "ifc/v1/disaridan/bsi-ifc2x3-building-architecture.ifc"),
]


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, path):
    return {"status": "failed", "error": {"code": code, "message": "$any", "path": path}}


def step(op, input_, result):
    return {"op": op, "input": input_, "result": result, "expect": dict(SAME)}


def case(name, steps):
    return {"name": name, "steps": steps}


def written_unit(u):
    """A unit as written: an SI unit's prefix and name, another's name; none for a unit with no name."""
    if u is None:
        return None
    if u.is_a("IfcSIUnit"):
        return (u.Prefix or "") + u.Name
    name = getattr(u, "Name", None)
    return name if isinstance(name, str) else None


def first(rels, attr):
    """Each object's first relation's other side (`attr`), by record number, the first relation in the file winning."""
    out = {}
    for r in sorted(rels, key=lambda r: r.id()):
        for o in getattr(r, attr[1]) or ():
            out.setdefault(o.id(), getattr(r, attr[0]))
    return out


def summary(f, rel):
    objects = f.by_type("IfcObjectDefinition")
    container = first(f.by_type("IfcRelContainedInSpatialStructure"), ("RelatingStructure", "RelatedElements"))
    storeys = []
    for s in sorted(f.by_type("IfcBuildingStorey"), key=lambda s: s.id()):
        contains = sum(1 for o in objects if container.get(o.id()) is not None and container[o.id()].id() == s.id())
        storeys.append({"globalId": s.GlobalId, "name": s.Name, "contains": contains})
    units = {k: written_unit(unit.get_project_unit(f, t)) for k, t in
             [("length", "LENGTHUNIT"), ("area", "AREAUNIT"), ("volume", "VOLUMEUNIT"), ("planeAngle", "PLANEANGLEUNIT")]}
    return {
        "path": f"$fixture:{rel}",
        "schema": f.schema_identifier,
        "records": len(list(f)),
        "objects": len(objects),
        "classes": dict(sorted(Counter(o.is_a() for o in objects).items())),
        "units": units,
        "storeys": storeys,
        "findings": [],
    }


QUANTITY = {"LengthValue": "IFCLENGTHMEASURE", "AreaValue": "IFCAREAMEASURE", "VolumeValue": "IFCVOLUMEMEASURE",
            "CountValue": "IFCCOUNTMEASURE", "WeightValue": "IFCMASSMEASURE", "TimeValue": "IFCTIMEMEASURE",
            "NumberValue": "IFCREAL"}


def shortest(x):
    """A number in words: the shortest decimal that reads back to it, without an exponent or a trailing `.0`."""
    from decimal import Decimal
    text = format(Decimal(repr(float(x))), "f")
    return text[:-2] if text.endswith(".0") else text


def typed(v):
    """A typed value as the command gives it."""
    if v is None:
        return {"kind": "empty"}
    w = v.wrappedValue
    if isinstance(w, bool):
        return {"kind": "logical", "logical": w}
    if w == "UNKNOWN" and v.is_a("IfcLogical"):
        return {"kind": "logical", "logical": None}
    if isinstance(w, float):
        return {"kind": "number", "number": w, "measure": v.is_a().upper()}
    if isinstance(w, int):
        return {"kind": "integer", "integer": w, "measure": v.is_a().upper()}
    return {"kind": "text", "text": str(w)}


def words(v):
    w = v.wrappedValue
    if isinstance(w, bool):
        return "true" if w else "false"
    if isinstance(w, float):
        return shortest(w)
    return str(w)


def value_of(p):
    """A property's or a quantity's value."""
    if p.is_a("IfcPropertySingleValue"):
        return typed(p.NominalValue)
    if p.is_a("IfcPropertyEnumeratedValue"):
        return {"kind": "enumeration", "items": [words(x) for x in p.EnumerationValues or ()]}
    if p.is_a("IfcPropertyListValue"):
        return {"kind": "list", "items": [words(x) for x in p.ListValues or ()]}
    if p.is_a("IfcPropertyBoundedValue"):
        lo, hi = p.LowerBoundValue, p.UpperBoundValue
        ends = [x for x in (lo, hi) if x is not None]
        return {"kind": "bounded", "lower": None if lo is None else float(lo.wrappedValue),
                "upper": None if hi is None else float(hi.wrappedValue),
                "measure": ends[0].is_a().upper() if ends else None}
    if p.is_a("IfcPropertyReferenceValue"):
        ref = p.PropertyReference
        return {"kind": "reference", "step": ref.id()} if ref is not None else {"kind": "empty"}
    for attr, measure in QUANTITY.items():
        if hasattr(p, attr):
            return {"kind": "number", "number": float(getattr(p, attr)), "measure": measure}
    return {"kind": "empty"}


def set_of(s, from_type):
    if s.is_a("IfcPropertySet"):
        kind, members = "properties", s.HasProperties or ()
    elif s.is_a("IfcElementQuantity"):
        kind, members = "quantities", s.Quantities or ()
    else:
        kind, members = "other", ()
    values = [{"name": m.Name, "class": m.is_a(), "value": value_of(m), "unit": written_unit(getattr(m, "Unit", None))}
              for m in members]
    return {"name": s.Name or "", "kind": kind, "fromType": from_type, "values": values}


def definitions(rel):
    d = rel.RelatingPropertyDefinition
    return list(d) if isinstance(d, tuple) else [d]


def details(f, o):
    """The object's attributes, its type's and storey's names, its own sets then its type's."""
    rels = sorted((r for r in f.get_inverse(o) if r.is_a("IfcRelDefinesByProperties")), key=lambda r: r.id())
    sets = [set_of(d, False) for r in rels for d in definitions(r)]
    sets += [set_of(d, False) for d in (getattr(o, "HasPropertySets", None) or ())]
    typing = [r for r in sorted(f.by_type("IfcRelDefinesByType"), key=lambda r: r.id()) if o in (r.RelatedObjects or ())]
    kind = typing[0].RelatingType if typing else None
    if kind is not None:
        sets += [set_of(d, True) for d in (kind.HasPropertySets or ())]
    parent = first(f.by_type("IfcRelAggregates") + f.by_type("IfcRelNests"), ("RelatingObject", "RelatedObjects"))
    container = first(f.by_type("IfcRelContainedInSpatialStructure"), ("RelatingStructure", "RelatedElements"))
    storey, at = None, o
    for _ in range(len(list(f))):
        at = parent.get(at.id()) or container.get(at.id())
        if at is None:
            break
        if at.is_a("IfcBuildingStorey"):
            storey = at.Name
            break
    attr = lambda name: getattr(o, name, None)  # noqa: E731
    object_type = attr("ObjectType") if o.is_a("IfcObject") else attr("ElementType")
    return {
        "globalId": o.GlobalId,
        "step": o.id(),
        "class": o.is_a(),
        "name": attr("Name"),
        "description": attr("Description"),
        "objectType": object_type,
        "longName": attr("LongName"),
        "tag": attr("Tag"),
        "predefinedType": attr("PredefinedType"),
        "typeName": kind.Name if kind is not None else None,
        "storey": storey,
        "sets": sets,
    }


SOUND = "ifc/v1/disaridan/bsi-ifc4-wall-with-opening-and-window.ifc"


def record_at(lines, step):
    """The line (from 0) record `step` is written on."""
    return next(i for i, l in enumerate(lines) if re.match(rf"#{step}\s*=", l))


STALE = []  # under --check: the damaged files on disk that are not what broken() makes


def put(path, text):
    """Writes a damaged file; under --check writes nothing and notes the file when it is not this text."""
    if "--check" in sys.argv:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            STALE.append(path.name)
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def broken():
    """Damaged files made from a sound one, and what is wrong in each by how it was made: name -> finding (none: the
    file is refused)."""
    sound = (ROOT / "fixtures" / SOUND).read_text(encoding="utf-8")
    lines = sound.splitlines(keepends=True)
    f = ifcopenshell.open(str(ROOT / "fixtures" / SOUND))
    out = {}
    put(BROKEN / "step-degil.ifc", "Bu bir IFC dosyası değil; bir metin dosyası.\n")
    out["step-degil"] = None
    schema = re.sub(r"FILE_SCHEMA\s*\(\s*\(\s*'IFC4'\s*\)\s*\)", "FILE_SCHEMA(('IFC9'))", sound, count=1)
    assert schema != sound
    put(BROKEN / "bilinmeyen-sema.ifc", schema)
    out["bilinmeyen-sema"] = None
    # A reference to a record the file does not hold: the window's representation made #999999.
    window = f.by_type("IfcWindow")[0]
    at = record_at(lines, window.id())
    rep = f"#{window.Representation.id()}"
    assert lines[at].count(rep) == 1
    dangling = lines[:at] + [lines[at].replace(rep, "#999999")] + lines[at + 1:]
    put(BROKEN / "olmayan-basvuru.ifc", "".join(dangling))
    out["olmayan-basvuru"] = {"code": "reference_missing", "severity": "error", "step": window.id(), "line": at + 1}
    # A GlobalId said twice: the wall's given to the window, the later record.
    wall = f.by_type("IfcWall")[0]
    assert window.id() > wall.id()
    at = record_at(lines, window.id())
    twice = lines[:at] + [lines[at].replace(window.GlobalId, wall.GlobalId, 1)] + lines[at + 1:]
    put(BROKEN / "ayni-globalid.ifc", "".join(twice))
    out["ayni-globalid"] = {"code": "global_id_duplicate", "severity": "error", "step": window.id(), "line": at + 1}
    return out


def cases():
    out = []
    opened = {}
    for title, rel in FILES:
        f = opened[rel] = ifcopenshell.open(str(ROOT / "fixtures" / rel))
        out.append(case(f"Özet: {title}", [step("execute", {"path": f"$fixture:{rel}"}, completed(summary(f, rel)))]))

    def asked(title, rel, o):
        f = opened[rel]
        output = summary(f, rel) | {"object": details(f, o)}
        return case(title, [step("execute", {"path": f"$fixture:{rel}", "object": o.GlobalId}, completed(output))])

    building = opened["arch/v1/ifc/kesit-binasi.ifc"]
    wall = building.by_guid("01alMXGa9m080000000001")
    out.append(asked("Bir dış duvar: öznitelikleri, Pset_WallCommon ve miktarları", "arch/v1/ifc/kesit-binasi.ifc", wall))
    space = sorted(building.by_type("IfcSpace"), key=lambda o: o.GlobalId)[0]
    out.append(asked("Bir mahal: kaplamaları ve alanları", "arch/v1/ifc/kesit-binasi.ifc", space))
    storey = sorted(building.by_type("IfcBuildingStorey"), key=lambda o: o.id())[0]
    out.append(asked("Bir kat: miktarları", "arch/v1/ifc/kesit-binasi.ifc", storey))
    other = opened["ifc/v1/acik/ifcopenshell-ifc4x3.ifc"]
    out.append(asked("Başka bir programın duvarı", "ifc/v1/acik/ifcopenshell-ifc4x3.ifc",
                     sorted(other.by_type("IfcWall"), key=lambda o: o.GlobalId)[0]))
    real = opened["ifc/v1/disaridan/bsi-ifc4x3-building-architecture.ifc"]
    out.append(asked("Gerçek örneğin duvarı", "ifc/v1/disaridan/bsi-ifc4x3-building-architecture.ifc",
                     sorted(real.by_type("IfcWall"), key=lambda o: o.GlobalId)[0]))
    # An object whose type has sets of its own: they come after its own, from the type.
    for _, rel in FILES:
        f = opened[rel]
        typed_objects = [o for r in sorted(f.by_type("IfcRelDefinesByType"), key=lambda r: r.id())
                         if r.RelatingType.HasPropertySets for o in r.RelatedObjects or ()]
        if typed_objects:
            out.append(asked("Tipinden gelen setler, kendi setlerinden sonra", rel, typed_objects[0]))
            break
    rel = "arch/v1/ifc/kesit-binasi.ifc"
    out.append(case("Planı: aynı özet, hiçbir şey yazılmaz",
                    [step("plan", {"path": f"$fixture:{rel}"}, completed(summary(building, rel)))]))
    out.append(case("Denetle: dosya okunuyor", [step("validate", {"path": f"$fixture:{rel}"}, completed(None))]))
    out.append(case("Yol boş", [step("execute", {"path": ""}, failed("invalid_file_path", "path"))]))
    out.append(case("Yol göreli", [step("execute", {"path": "bina.ifc"}, failed("invalid_file_path", "path"))]))
    out.append(case(".ifc değil", [step("execute", {"path": "$fixture:arch/v1/section-building.kcad"},
                                       failed("invalid_file_path", "path"))]))
    out.append(case("Dosya yok", [step("execute", {"path": "$fixture:ifc/v1/yok.ifc"}, failed("invalid_file_path", "path"))]))
    out.append(case("Nesne dosyada yok", [step("execute", {"path": f"$fixture:{rel}", "object": "0000000000000000000000"},
                                              failed("ifc_object_not_found", "object"))]))
    damage = broken()
    out.append(case("STEP dosyası değil", [step("execute", {"path": "$fixture:ifc/v1/bozuk/step-degil.ifc"},
                                               failed("ifc_unreadable", "path"))]))
    out.append(case("Şeması tanınmıyor", [step("execute", {"path": "$fixture:ifc/v1/bozuk/bilinmeyen-sema.ifc"},
                                              failed("ifc_unreadable", "path"))]))
    sound_rel = SOUND
    for name, title in [("olmayan-basvuru", "Olmayan kayda başvuru: okunur, bulgusu satırıyla"),
                        ("ayni-globalid", "İki nesnede aynı GlobalId: okunur, bulgusu satırıyla")]:
        rel = f"ifc/v1/bozuk/{name}.ifc"
        f = ifcopenshell.open(str(ROOT / "fixtures" / rel))
        expected = summary(f, rel)
        if name == "olmayan-basvuru":
            # The placement a reader cannot find is empty: the file's other counts are the sound one's.
            expected = summary(opened[sound_rel], rel)
        expected["findings"] = [damage[name] | {"message": "$any"}]
        warning = {"code": "ifc_read_findings", "message": "$any"}
        out.append(case(title, [step("execute", {"path": f"$fixture:{rel}"}, completed(expected, [warning]))]))
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "cad.ifc.inspect", "commandVersion": 1,
            "title": "IFC dosyasını incele",
            "note": "scripts/fixtures/ifc_inspect_cases.py yazar; beklenenler IfcOpenShell 0.9.0.post1'in aynı dosyaları "
                    "okuyuşundan, KentOS kodu ve KentOS'un okuma motoru olmadan; bozuk dosyalar sağlam birinden betikte "
                    "yapılır, bulgunun kaydı ve satırı yapılışından bilinir.",
            "setup": SETUP, "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if STALE:
            raise SystemExit(f"{BROKEN} güncel değil ({', '.join(STALE)}): betiği --check olmadan çalıştırıp farkı okuyun.")
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil: betiği --check olmadan çalıştırıp farkı okuyun.")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
