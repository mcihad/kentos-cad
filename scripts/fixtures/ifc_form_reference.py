#!/usr/bin/env python3
"""EK-9's form on another reader's facts (plan/ifc-uyum.md IFC-08 "İlk kabul", docs/adr/8074 §7): IfcOpenShell (not
KentOS code, not KentOS's reading engine) reads KentOS's files and gives the facts the form reads, each by its own means:

- the objects (products, the project, groups) with their storey (containment and aggregation walked up), attributes,
  own and type sets, quantities and materials; the storeys' and the site's elevations;
- EK-7's records by the schema's attribute names;
- the mandatory rows the objects do not carry, the rule read from the catalog's data
  (crates/shared/ifc/data/temel-2026.json; an attribute, a set's property or quantity, the material; a predefined type
  NOTDEFINED is none, an empty word is none);
- the standard sets against IFC 4.3's templates, IfcOpenShell's own;
- the bodies from its own triangulation: heights and plan boxes, a space closed when every edge is two triangles', the
  ground two spaces of a storey share summed triangle by triangle; a body it cannot make, with why;
- each product's shape representations as written (identifier, type, a mapped one's the map's, a tessellated one's
  closure), whether it is a building element (not an opening or a virtual boundary) and what it is a part of: the
  level of development EK-9/7 reads (docs/adr/8133).

The standards package's form answers them (the rule under test is the same for both readers; the readers are not).
Writes fixtures/ifc/v1/form/<name>.json: each question's evidence and its findings (code and GlobalIds), which
catalog/standards/tests/dijital-proje/test_form.py holds KentOS's own reading to; --check compares instead. Runs under
IfcOpenShell, offline:

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 \\
        python -I scripts/fixtures/ifc_form_reference.py [--check]
"""

import json
import sys
from pathlib import Path

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element as element
import ifcopenshell.util.pset as pset_templates

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "ifc" / "v1" / "form"
CATALOG = json.loads((ROOT / "crates/shared/ifc/data/temel-2026.json").read_text(encoding="utf-8"))
sys.path.insert(0, str(ROOT / "catalog/standards/dijital-proje/src"))
from dijital_proje import form  # noqa: E402

FILES = {
    "kesit-binasi": "fixtures/arch/v1/ifc/kesit-binasi.ifc",
    "kesit-binasi-kunye": "fixtures/arch/v1/ifc/kesit-binasi-kunye.ifc",
    "kesit-binasi-tam": "fixtures/arch/v1/ifc/kesit-binasi-tam.ifc",
    "yerel-sistem": "fixtures/arch/v1/ifc/yerel-sistem.ifc",
    "kesit-binasi-kapi-eksik": "fixtures/ifc/v1/eksik/kesit-binasi-kapi-eksik.ifc",
}
RECORDS = {"IfcPerson", "IfcPersonAndOrganization", "IfcOrganization", "IfcActorRole", "IfcPostalAddress",
           "IfcProjectedCRS", "IfcMapConversion", "IfcGeometricRepresentationContext"}
# Two corners are the same when written alike to this many metres (the triangulation's own corners).
WELD = 1e-6


def plain(v):
    """An attribute's value as the facts write it: a record a reference, an enumeration its word."""
    if isinstance(v, ifcopenshell.entity_instance):
        if v.id() == 0:
            return plain(v.wrappedValue)
        return f"#{v.id()}"
    if isinstance(v, (list, tuple)):
        return [plain(x) for x in v]
    return v


def value(p):
    """A property's value, or none when it holds nothing."""
    if p.is_a("IfcPropertySingleValue"):
        return None if p.NominalValue is None else p.NominalValue.wrappedValue
    if p.is_a("IfcPropertyEnumeratedValue"):
        return [x.wrappedValue for x in p.EnumerationValues or []] or None
    if p.is_a("IfcPropertyListValue"):
        return [x.wrappedValue for x in p.ListValues or []] or None
    if p.is_a("IfcPropertyBoundedValue"):
        ends = [x.wrappedValue for x in (p.LowerBoundValue, p.UpperBoundValue) if x is not None]
        return ends or None
    if p.is_a("IfcPhysicalSimpleQuantity"):
        return p[3]
    return None


def sets_of(definitions):
    """Property sets and quantity sets by name: {set: {name: value}}, and which are quantities."""
    sets, quantities = {}, {}
    for d in definitions:
        if d.is_a("IfcPropertySet"):
            sets[d.Name] = {p.Name: value(p) for p in d.HasProperties or []}
        elif d.is_a("IfcElementQuantity"):
            quantities[d.Name] = {q.Name: value(q) for q in d.Quantities or []}
    return sets, quantities


def own_definitions(o):
    return [r.RelatingPropertyDefinition for r in getattr(o, "IsDefinedBy", None) or []
            if r.is_a("IfcRelDefinesByProperties")]


def type_of(o):
    for r in getattr(o, "IsTypedBy", None) or []:
        return r.RelatingType
    return None


def materials(o):
    m = element.get_material(o, should_skip_usage=True)
    if m is None:
        return []
    if m.is_a("IfcMaterial"):
        return [m.Name]
    if m.is_a("IfcMaterialLayerSet"):
        return [layer.Material.Name for layer in m.MaterialLayers if layer.Material]
    if m.is_a("IfcMaterialConstituentSet"):
        return [c.Material.Name for c in m.MaterialConstituents or [] if c.Material]
    if m.is_a("IfcMaterialProfileSet"):
        return [p.Material.Name for p in m.MaterialProfiles or [] if p.Material]
    if m.is_a("IfcMaterialList"):
        return [x.Name for x in m.Materials]
    return []


def up(o):
    """The structure an object is contained in or a part of."""
    for r in getattr(o, "ContainedInStructure", None) or []:
        return r.RelatingStructure
    for r in getattr(o, "Decomposes", None) or []:
        return r.RelatingObject
    return None


def storey_of(o):
    at = up(o)
    while at is not None:
        if at.is_a("IfcBuildingStorey"):
            return at.GlobalId
        at = up(at)
    return None


def attribute(o, name):
    try:
        return getattr(o, name)
    except AttributeError:
        return None


def kind_of(rep, identifier, depth=0):
    """A shape representation's identifier and type; a mapped one's type that of the representation its first item
    maps; a tessellated one closed when every face set says so, open when one says not."""
    t = rep.RepresentationType
    items = list(rep.Items or [])
    if t == "MappedRepresentation" and depth < 4 and items and items[0].is_a("IfcMappedItem"):
        return kind_of(items[0].MappingSource.MappedRepresentation, identifier, depth + 1)
    closed = None
    if t == "Tessellation":
        said = [getattr(i, "Closed", None) for i in items]
        closed = False if False in said else (None if not said or None in said else True)
    return {"identifier": identifier, "type": t, "closed": closed}


def shapes_of(o):
    r = getattr(o, "Representation", None)
    if r is None:
        return []
    return [kind_of(x, x.RepresentationIdentifier) for x in r.Representations or []]


def parent_of(o):
    """What it is a part of (IfcRelAggregates, IfcRelNests), by GlobalId."""
    for rel in list(getattr(o, "Decomposes", None) or []) + list(getattr(o, "Nests", None) or []):
        return rel.RelatingObject.GlobalId
    return None


def objects_of(f):
    out = []
    for o in f.by_type("IfcRoot"):
        if not (o.is_a("IfcProduct") or o.is_a("IfcProject") or o.is_a("IfcGroup")) or o.is_a("IfcTypeObject"):
            continue
        sets, quantities = sets_of(own_definitions(o))
        t = type_of(o)
        type_sets, type_quantities = sets_of(t.HasPropertySets or []) if t is not None else ({}, {})
        elevation = None
        if o.is_a("IfcBuildingStorey"):
            elevation = o.Elevation
        elif o.is_a("IfcSite"):
            elevation = o.RefElevation
        predefined = attribute(o, "PredefinedType")
        out.append({
            "globalId": o.GlobalId, "class": o.is_a(), "name": o.Name, "longName": attribute(o, "LongName"),
            "description": o.Description, "objectType": attribute(o, "ObjectType"),
            "predefinedType": predefined, "tag": attribute(o, "Tag"), "storey": storey_of(o),
            "elevation": elevation, "sets": sets, "typeSets": type_sets, "quantities": quantities,
            "typeQuantities": type_quantities, "materials": materials(o),
            "element": o.is_a("IfcElement") and not o.is_a("IfcFeatureElement") and not o.is_a("IfcVirtualElement"),
            "shapes": shapes_of(o), "parent": parent_of(o),
        })
    return out


def records_of(f):
    """EK-7's records, each class itself (a context's sub-contexts derive their fields), a map conversion with its
    scaled kind (a local system's)."""
    out = {}
    for name in sorted(RECORDS):
        for e in f.by_type(name, include_subtypes=(name == "IfcMapConversion")):
            info = e.get_info(include_identifier=False, recursive=False)
            info.pop("type", None)
            out.setdefault(name, []).append({"step": e.id(), "class": e.is_a(),
                                             "attributes": {k: plain(v) for k, v in info.items()}})
    return out


def given(v):
    if v is None:
        return False
    if isinstance(v, str):
        return bool(v.strip())
    if isinstance(v, (list, tuple)):
        return any(given(x) for x in v)
    return True


def carries(o, r):
    """Whether an object carries a mandatory row; none where the rule does not judge it (KentOS's `required` too)."""
    kind = r["kind"]
    if kind in ("property", "quantity"):
        where = ("sets", "typeSets") if kind == "property" else ("quantities", "typeQuantities")
        held = [s.get(r["set"], {}) for s in (o[w] for w in where)]
        if kind == "quantity":
            return any(r["name"] in h for h in held)
        return any(given(h.get(r["name"])) for h in held)
    if kind == "material":
        return given(o["materials"][:1])
    if kind == "attribute":
        name = r["name"].replace("Attribute:", "").strip()
        fields = {"Name": "name", "LongName": "longName", "Description": "description", "ObjectType": "objectType",
                  "Tag": "tag", "AxisTag": "tag"}
        if name == "PredefinedType":
            p = o["predefinedType"]
            return bool(p) and p != "NOTDEFINED"
        if name == "RefElevation":
            return o["elevation"] is not None
        if name in fields:
            return given(o[fields[name]])
    return None


# EK-6 Tablo 6.7 asks the building's Height and EavesHeight "sadece yüksekliğin sabit olduğu durumda" (IFC 4.3: "only
# provided if there is a constant height"): read here by the storeys' own GrossHeight and NetHeight in the file.
CONSTANT = {"Height": "GrossHeight", "EavesHeight": "NetHeight"}


def not_constant(objects, name):
    """Whether the file's storeys carry differing heights of `name`: then the building has no constant height."""
    known = [((o.get("quantities") or {}).get("Qto_BuildingStoreyBaseQuantities") or {}).get(name)
             for o in objects if o["class"].lower() == "ifcbuildingstorey"]
    known = [h for h in known if isinstance(h, (int, float))]
    return any(abs(h - known[0]) > 1e-9 for h in known)


def gaps_of(objects):
    out = []
    for r in CATALOG["requirements"]:
        if r["level"] != "mandatory":
            continue
        of = [o for o in objects if o["class"].lower() == r["class"].lower()]
        if not of:
            continue
        if (r["kind"] == "quantity" and r["class"].lower() == "ifcbuilding"
                and r.get("set") == "Qto_BuildingBaseQuantities" and r["name"] in CONSTANT
                and not_constant(objects, CONSTANT[r["name"]])):
            continue
        judged = [(o, carries(o, r)) for o in of]
        lacking = [o["globalId"] for o, c in judged if c is False]
        if lacking and any(c is not None for _, c in judged):
            path = f"{r['set']}.{r['name']}" if r["kind"] in ("property", "quantity") else (
                "IfcMaterial.Name" if r["kind"] == "material" else r["name"])
            out.append({"id": r["id"], "class": r["class"], "path": path, "tr": r["tr"], "table": r["table"],
                        "page": r["page"], "missing": lacking, "of": len(of)})
    return out


def is_sub(schema, cls, base):
    d = schema.declaration_by_name(cls)
    while d is not None:
        if d.name().lower() == base.lower():
            return True
        d = d.supertype()
    return False


def set_checks(f, objects):
    schema = ifcopenshell.ifcopenshell_wrapper.schema_by_name(f.schema_identifier)
    templates = pset_templates.get_template("IFC4X3")
    out = []
    by = {o.GlobalId: o for o in f.by_type("IfcRoot")}
    for o in objects:
        e = by[o["globalId"]]
        t = type_of(e)
        for from_type, group in ((False, ("sets", "quantities")), (True, ("typeSets", "typeQuantities"))):
            cls = t.is_a() if from_type and t is not None else o["class"]
            for g in group:
                for name, values in o[g].items():
                    if not (name.startswith("Pset_") or name.startswith("Qto_")):
                        continue
                    tpl = templates.get_by_name(name)
                    if tpl is None:
                        out.append({"globalId": o["globalId"], "set": name, "item": None, "why": "unknown_set"})
                        continue
                    applies = [a.split("/")[0].strip() for a in (tpl.ApplicableEntity or "").split(",") if a.strip()]
                    if applies and not any(is_sub(schema, cls, a) for a in applies):
                        out.append({"globalId": o["globalId"], "set": name, "item": None, "why": "not_for_class"})
                    known = {p.Name for p in tpl.HasPropertyTemplates or []}
                    for item in values:
                        if item not in known:
                            out.append({"globalId": o["globalId"], "set": name, "item": item, "why": "unknown_item"})
    return out


def area(poly):
    return abs(sum(poly[i][0] * poly[i - 1][1] - poly[i - 1][0] * poly[i][1] for i in range(len(poly)))) / 2.0


def clip(subject, a, b):
    """The part of a convex polygon on the left of the line a→b (Sutherland–Hodgman, one edge)."""
    out = []
    side = lambda p: (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])  # noqa: E731
    for i, p in enumerate(subject):
        q = subject[(i + 1) % len(subject)]
        sp, sq = side(p), side(q)
        if sp >= 0:
            out.append(p)
        if (sp >= 0) != (sq >= 0):
            t = sp / (sp - sq)
            out.append((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t))
    return out


def shared(t, u):
    """The area two triangles in plan share."""
    if area(t) == 0 or area(u) == 0:
        return 0.0
    if (t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0]) < 0:
        t = t[::-1]
    poly = list(u)
    for i in range(3):
        if len(poly) < 3:
            return 0.0
        poly = clip(poly, t[i], t[(i + 1) % 3])
    return area(poly) if len(poly) >= 3 else 0.0


def bodies_of(f, objects):
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    storeys = {o["globalId"]: o["storey"] for o in objects}
    bodies, spaces, grounds, not_shown = [], [], [], []
    for e in f.by_type("IfcProduct"):
        if e.is_a("IfcOpeningElement") or e.Representation is None:
            continue
        if not any(r.RepresentationIdentifier == "Body" for r in e.Representation.Representations):
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, e)
        except RuntimeError as why:
            not_shown.append({"globalId": e.GlobalId, "reason": str(why)})
            continue
        v = shape.geometry.verts
        pts = [(v[i], v[i + 1], v[i + 2]) for i in range(0, len(v), 3)]
        tri = shape.geometry.faces
        tris = [(tri[i], tri[i + 1], tri[i + 2]) for i in range(0, len(tri), 3)]
        if not pts:
            continue
        lo, hi = min(p[2] for p in pts), max(p[2] for p in pts)
        box = [min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts)]
        bodies.append({"globalId": e.GlobalId, "zMin": lo, "zMax": hi, "box": box, "rounding": WELD})
        if e.is_a("IfcSpace"):
            key = lambda i: tuple(round(c / WELD) for c in pts[i])  # noqa: E731
            edges = {}
            for t in tris:
                for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
                    ka, kb = key(a), key(b)
                    if ka != kb:
                        edges[tuple(sorted((ka, kb)))] = edges.get(tuple(sorted((ka, kb))), 0) + 1
            closed = bool(edges) and all(n == 2 for n in edges.values())
            ground = [tuple((pts[i][0], pts[i][1]) for i in t) for t in tris
                      if all(abs(pts[i][2] - lo) <= 2 * WELD for i in t)]
            spaces.append({"globalId": e.GlobalId, "storey": storeys.get(e.GlobalId), "area": sum(area(g) for g in ground),
                           "zMin": lo, "zMax": hi, "rounding": WELD, "closed": closed})
            grounds.append((e.GlobalId, storeys.get(e.GlobalId), ground, box))
    overlaps = []
    for i, (a, sa, ga, ba) in enumerate(grounds):
        for b, sb, gb, bb in grounds[i + 1:]:
            if sa != sb or ba[2] <= bb[0] or bb[2] <= ba[0] or ba[3] <= bb[1] or bb[3] <= ba[1]:
                continue
            s = sum(shared(t, u) for t in ga for u in gb)
            if s > 0:
                overlaps.append({"a": a, "b": b, "area": s})
    return {"bodies": bodies, "spaces": spaces, "overlaps": overlaps, "notShown": not_shown}


def facts(name, path):
    f = ifcopenshell.open(str(ROOT / path))
    objects = objects_of(f)
    seen, findings = {}, []
    for o in f.by_type("IfcRoot"):
        if o.GlobalId in seen:
            findings.append({"code": "global_id_duplicate", "severity": "error", "step": o.id(), "line": 0,
                             "message": f"“{o.GlobalId}” GlobalId'si iki nesnede: #{seen[o.GlobalId]} ve #{o.id()}."})
        seen.setdefault(o.GlobalId, o.id())
    header = f.header.file_name
    return {
        "format": "kentos.ifcFacts", "version": 1,
        "file": {"name": Path(path).name, "extension": Path(path).suffix.lstrip(".").lower(),
                 "bytes": (ROOT / path).stat().st_size, "schema": f.schema_identifier,
                 "header": {"name": header.name}},
        "objects": objects,
        "records": records_of(f),
        "rows": [{"id": r["id"], "table": r["table"], "page": r["page"], "class": r["class"], "name": r["name"],
                  "path": r["name"], "tr": r["tr"], "mandatory": r["level"] == "mandatory",
                  "attribute": r["kind"] == "attribute"}
                 for r in CATALOG["requirements"] if str(r["table"]).startswith("7.")],
        "gaps": gaps_of(objects),
        "setChecks": set_checks(f, objects) if f.schema_identifier == "IFC4X3_ADD2" else [],
        "profile": {"schema": "IFC4X3_ADD2"},
        "classCodes": {c["class"]: c["code"] for c in CATALOG.get("classes", [])
                       if c["class"].lower() in {o["class"].lower() for o in objects} and c.get("code")},
        "findings": findings,
        "bodies": bodies_of(f, objects),
    }


def answers(f):
    """The form's 21 rows as the reference keeps them: evidence and findings (code, GlobalIds)."""
    rows = form.cevapla(f)["satirlar"]
    return [{"no": r["no"], "kanit": r["kanit"],
             "bulgular": sorted([b["code"], sorted(b.get("globalIds") or [])] for b in r["bulgular"])} for r in rows]


def main():
    check = "--check" in sys.argv
    OUT.mkdir(parents=True, exist_ok=True)
    for name, path in FILES.items():
        got = answers(facts(name, path))
        text = json.dumps({"file": path, "reader": f"IfcOpenShell {ifcopenshell.version}", "rows": got},
                          ensure_ascii=False, indent=1) + "\n"
        out = OUT / f"{name}.json"
        if check:
            if not out.is_file() or out.read_text(encoding="utf-8") != text:
                sys.exit(f"{out.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırıp farkı okuyun.")
        else:
            out.write_text(text, encoding="utf-8")
        print(name, " ".join(f"{r['no']}:{r['kanit'][0]}" for r in got))


if __name__ == "__main__":
    main()
