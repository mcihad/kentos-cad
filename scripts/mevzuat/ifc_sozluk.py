#!/usr/bin/env python3
"""IFC 4.3's dictionary for what the rule catalog names (docs/adr/8062; plan/ifc-uyum.md IFC-01): every class the
catalog names (EK-5's, EK-6's and EK-7's tables', the relations' and the material's) with its attributes in order, each
attribute's type and whether it is optional, and its predefined types; and every property and quantity set that
the catalog names, with its properties' or quantities' names, kinds and types and the classes it applies to, and the
names of all the others (a misspelled set's nearest name is among them). kentos-ifc compares its catalog
with this file (`catalog::schema`), so the comparison runs without IfcOpenShell.

Read from IfcOpenShell's IFC4X3_ADD2 schema (buildingSMART's EXPRESS) and its IFC 4.3 property set templates
(buildingSMART's templates, as IfcOpenShell ships them). A name the catalog writes and the schema does not have is
not here: that is what the comparison finds.

    uv run --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 \\
        python3 scripts/mevzuat/ifc_sozluk.py [--check]
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import ifcopenshell
import ifcopenshell.util.pset as psets

ROOT = Path(__file__).resolve().parents[2]
CATALOG = ROOT / "crates" / "shared" / "ifc" / "data" / "temel-2026.json"
OUT = ROOT / "crates" / "shared" / "ifc" / "data" / "ifc4x3-sozluk.json"
SCHEMA = "IFC4X3_ADD2"


def type_name(t) -> str:
    """An attribute's type as the schema names it: a named type's name, an aggregate's “LIST OF IfcLabel”."""
    kind = t.as_named_type() if hasattr(t, "as_named_type") else None
    if kind is not None:
        return kind.declared_type().name()
    agg = t.as_aggregation_type() if hasattr(t, "as_aggregation_type") else None
    if agg is not None:
        return f"{agg.type_of_aggregation_string().upper()} OF {type_name(agg.type_of_element())}"
    sel = t.as_simple_type() if hasattr(t, "as_simple_type") else None
    if sel is not None:
        return sel.declared_type()
    return str(t)


def enum_items(schema, attr_type) -> tuple[str | None, list[str]]:
    named = attr_type.as_named_type() if hasattr(attr_type, "as_named_type") else None
    if named is None:
        return None, []
    decl = named.declared_type()
    if hasattr(decl, "enumeration_items"):
        return decl.name(), list(decl.enumeration_items())
    return None, []


def main(argv: list[str]) -> int:
    check = "--check" in argv
    schema = ifcopenshell.ifcopenshell_wrapper.schema_by_name(SCHEMA)
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))

    wanted = {c["class"] for c in catalog["classes"]} | {t["class"] for t in catalog["tables"]}
    for r in catalog["requirements"]:
        for part in (r.get("path") or "").split(">"):
            if part.startswith("Ifc"):
                wanted.add(part)
    wanted |= {"IfcMaterial", "IfcRelAssociatesMaterial", "IfcRelAssignsToGroup", "IfcSystem", "IfcGrid"}

    classes, enums = {}, {}
    for name in sorted(wanted):
        try:
            decl = schema.declaration_by_name(name)
        except Exception:
            continue  # not IFC 4.3's: the comparison says so
        if not hasattr(decl, "all_attributes"):
            continue
        attrs = []
        predefined = None
        for a in decl.all_attributes():
            attrs.append([a.name(), type_name(a.type_of_attribute()), bool(a.optional())])
            if a.name() == "PredefinedType":
                enum, items = enum_items(schema, a.type_of_attribute())
                if enum:
                    predefined = enum
                    enums[enum] = items
        sup = decl.supertype()
        chain = []
        s = sup
        while s is not None:
            chain.append(s.name())
            s = s.supertype()
        classes[decl.name()] = {
            "supertypes": chain,
            "abstract": bool(decl.is_abstract()),
            "attributes": attrs,
            "predefined": predefined,
        }

    templates = psets.get_template("IFC4X3")
    sets = {}
    all_names = set()
    named = {r["set"] for r in catalog["requirements"] if r.get("set")}
    for t in templates.templates:
        for st in t.by_type("IfcPropertySetTemplate"):
            all_names.add(st.Name)
            applies = [e.strip() for e in (st.ApplicableEntity or "").split(",") if e.strip()]
            if st.Name not in named:
                continue  # its name alone, below: a misspelled set's nearest name is looked up among them
            items = []
            for p in st.HasPropertyTemplates or ():
                if not p.is_a("IfcSimplePropertyTemplate"):
                    items.append([p.Name, "COMPLEX", None])
                    continue
                kind = p.TemplateType or ""
                measure = p.PrimaryMeasureType
                values = None
                if p.Enumerators is not None:
                    values = [v.wrappedValue for v in p.Enumerators.EnumerationValues]
                item = [p.Name, kind, measure]
                if values:
                    item.append(values)
                items.append(item)
            sets[st.Name] = {
                "kind": "qto" if st.Name.startswith("Qto_") else "pset",
                "applies": applies,
                "items": items,
            }

    data = {
        "format": 1,
        "schema": SCHEMA,
        "source": {
            "ifcopenshell": ifcopenshell.version,
            "templates": "IFC 4.3 property and quantity set templates (buildingSMART), as IfcOpenShell ships them",
        },
        "classes": classes,
        "enums": dict(sorted(enums.items())),
        "sets": dict(sorted(sets.items())),
        "setNames": sorted(all_names),
    }
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if check:
        have = OUT.read_text(encoding="utf-8") if OUT.exists() else None
        if have != text:
            print(f"ifc_sozluk: {OUT.relative_to(ROOT)} is not what IfcOpenShell {ifcopenshell.version} gives", file=sys.stderr)
            return 1
    else:
        OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(classes)} sınıf, {len(enums)} tip listesi, {len(sets)} set, {len(text.encode())} bayt")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
