#!/usr/bin/env python3
"""Independent reference of the IFC files KentOS writes (and of two another program writes), read by another reader (docs/adr/8061 §7; plan/ifc-uyum.md
IFC-07's first acceptance: "yazılan her örnek geri okununca aynı nesne çizgesini verir").

For every file below, IfcOpenShell (not KentOS code, not the reading engine) gives: the schema and the header, the
number of records, the project's length unit, the map conversion, every object (each IfcObjectDefinition) with its
class, names, predefined type, whether it has a shape, and its place in the tree as the relations say it (the
aggregating or nesting object, the containing structure, the type), the own property and quantity sets of the first
object of each class (sorted by GlobalId), and every body's world bounding box, volume and area with openings cut
(ifcopenshell.geom, world coordinates), with the file's precision. kentos-ifc's `read` tests compare its reading face
with the graph and the sets; its `geometry` tests compare the engine's bodies with the boxes and volumes.

Writes fixtures/ifc/v1/okuma/<name>.json; --check compares instead. Runs under IfcOpenShell, offline:

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 python -I scripts/fixtures/ifc_read_cases.py
"""

import argparse
import json
import sys
from pathlib import Path

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element as element
import ifcopenshell.util.shape as shape
import ifcopenshell.util.unit as unit

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "ifc" / "v1" / "okuma"
FILES = {
    "kesit-binasi": "fixtures/arch/v1/ifc/kesit-binasi.ifc",
    "kesit-binasi-kunye": "fixtures/arch/v1/ifc/kesit-binasi-kunye.ifc",
    "blok-a": "fixtures/arch/v1/ifc/blok-a.ifc",
    "blok-b": "fixtures/arch/v1/ifc/blok-b.ifc",
    "cati-alti-mahal": "fixtures/arch/v1/ifc/cati-alti-mahal.ifc",
    "egimli-doseme": "fixtures/arch/v1/ifc/egimli-doseme.ifc",
    "egri-bina": "fixtures/arch/v1/ifc/egri-bina.ifc",
    "egri-bina-referans": "fixtures/arch/v1/ifc/egri-bina-referans.ifc",
    "katmanli-cati": "fixtures/arch/v1/ifc/katmanli-cati.ifc",
    "yerel-sistem": "fixtures/arch/v1/ifc/yerel-sistem.ifc",
    "yol": "fixtures/road/v1/ifc/out/yol.ifc",
    # Another writer's files (scripts/fixtures/ifc_other_writer_cases.py): IFC4 and IFC 4.3.
    "ifcopenshell-ifc4": "fixtures/ifc/v1/acik/ifcopenshell-ifc4.ifc",
    "ifcopenshell-ifc4x3": "fixtures/ifc/v1/acik/ifcopenshell-ifc4x3.ifc",
}


def value(v):
    """A property's value as JSON: IfcOpenShell's Python value, a tuple or a list as a list."""
    if isinstance(v, (tuple, list)):
        return [value(x) for x in v]
    if isinstance(v, (str, int, float, bool)) or v is None:
        return v
    return str(v)


def relations(f):
    """Each object's parent (aggregates, nests), container and type, by GlobalId, the first relation winning."""
    parent, container, typed = {}, {}, {}
    for r in f.by_type("IfcRelAggregates") + f.by_type("IfcRelNests"):
        for o in r.RelatedObjects or ():
            parent.setdefault(o.id(), r.RelatingObject.id())
    for r in f.by_type("IfcRelContainedInSpatialStructure"):
        for o in r.RelatedElements or ():
            container.setdefault(o.id(), r.RelatingStructure.id())
    for r in f.by_type("IfcRelDefinesByType"):
        for o in r.RelatedObjects or ():
            typed.setdefault(o.id(), r.RelatingType.id())
    return parent, container, typed


def attr(o, name):
    try:
        return getattr(o, name)
    except AttributeError:
        return None


def objects(f):
    parent, container, typed = relations(f)
    gid = {o.id(): o.GlobalId for o in f.by_type("IfcObjectDefinition")}
    out = {}
    for o in f.by_type("IfcObjectDefinition"):
        object_type = attr(o, "ObjectType") if o.is_a("IfcObject") else attr(o, "ElementType")
        predefined = attr(o, "PredefinedType")
        out[o.GlobalId] = {
            "step": o.id(),
            "class": o.is_a(),
            "name": attr(o, "Name"),
            "description": attr(o, "Description"),
            "objectType": object_type,
            "longName": attr(o, "LongName"),
            "tag": attr(o, "Tag"),
            "predefined": predefined,
            "hasShape": bool(attr(o, "Representation")) if o.is_a("IfcProduct") else False,
            "parent": gid.get(parent.get(o.id())),
            "container": gid.get(container.get(o.id())),
            "type": gid.get(typed.get(o.id())),
        }
    return out


def sets(f):
    """The own sets of the first object of each class (by GlobalId), their values by name."""
    first = {}
    for o in sorted(f.by_type("IfcObjectDefinition"), key=lambda o: o.GlobalId):
        first.setdefault(o.is_a(), o)
    out = {}
    for o in first.values():
        psets = element.get_psets(o, should_inherit=False)
        if psets:
            out[o.GlobalId] = {
                name: {k: value(v) for k, v in props.items() if k != "id"} for name, props in sorted(psets.items())
            }
    return out


def bodies(f):
    """Every product's body in world coordinates, openings cut: its box, volume and area."""
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for p in f.by_type("IfcProduct"):
        if not p.Representation or p.is_a("IfcOpeningElement"):
            continue
        try:
            s = ifcopenshell.geom.create_shape(settings, p)
        except RuntimeError:
            continue
        v = s.geometry.verts
        if not v:
            continue
        xs, ys, zs = v[0::3], v[1::3], v[2::3]
        out[p.GlobalId] = {
            "box": [[min(xs), min(ys), min(zs)], [max(xs), max(ys), max(zs)]],
            "volume": shape.get_volume(s.geometry),
            "area": shape.get_area(s.geometry),
        }
    return out


def location(f):
    conversions = f.by_type("IfcMapConversion")
    if not conversions:
        return None
    c = conversions[0]
    t = c.TargetCRS
    out = {
        "eastings": c.Eastings,
        "northings": c.Northings,
        "orthogonalHeight": c.OrthogonalHeight,
        "xAxisAbscissa": c.XAxisAbscissa,
        "xAxisOrdinate": c.XAxisOrdinate,
        "scale": c.Scale,
        "crs": t.Name if t else None,
    }
    if c.is_a("IfcMapConversionScaled"):
        out["factors"] = [c.FactorX, c.FactorY, c.FactorZ]
    return out


def case(path):
    f = ifcopenshell.open(str(ROOT / path))
    name = f.header.file_name
    contexts = [c for c in f.by_type("IfcGeometricRepresentationContext") if not c.is_a("IfcGeometricRepresentationSubContext")]
    return {
        "source": path,
        "reader": f"IfcOpenShell {ifcopenshell.version}",
        "schema": f.schema_identifier,
        "header": {
            "name": name.name,
            "timeStamp": name.time_stamp,
            "preprocessor": name.preprocessor_version,
            "originatingSystem": name.originating_system,
        },
        "records": len(list(f)),
        "lengthToSi": unit.calculate_unit_scale(f),
        "precision": contexts[0].Precision if contexts else None,
        "location": location(f),
        "objects": objects(f),
        "sets": sets(f),
        "bodies": bodies(f),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the files instead of writing them")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    stale = []
    for name, path in FILES.items():
        text = json.dumps(case(path), ensure_ascii=False, indent=1, sort_keys=True) + "\n"
        out = OUT / f"{name}.json"
        if args.check:
            if not out.exists() or out.read_text(encoding="utf-8") != text:
                stale.append(out)
        else:
            out.write_text(text, encoding="utf-8")
            print(out.relative_to(ROOT))
    if stale:
        for s in stale:
            print(f"{s.relative_to(ROOT)} IfcOpenShell'in okuduğuyla aynı değil; betiği --check olmadan çalıştırıp farkı okuyun.")
        sys.exit(1)


if __name__ == "__main__":
    main()
