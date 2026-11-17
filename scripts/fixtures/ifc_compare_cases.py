#!/usr/bin/env python3
"""Shared cases of `cad.ifc.compare` (plan/ifc-uyum.md IFC-10, docs/adr/8103), written without KentOS code: IfcOpenShell
writes a small model (two storeys, a wall type, walls, a column, a slab, a door without a body, a room) and its
revisions, each with known changes, and reads them back; the expected comparison is made here by the ADR's rules over
IfcOpenShell's reading (the attributes and sets as scripts/fixtures/ifc_inspect_cases.py reads them, the bodies' boxes by
IfcOpenShell's geometry):

- objects are every IfcObjectDefinition, in the file's order, matched by GlobalId alone; one a file holds twice is left
  out and listed (`repeated`, the older's then the newer's, by GlobalId);
- added: in the newer only (its order); removed: in the older only (its order); changed: both, with one of: the class;
  Name, Description, ObjectType, LongName, Tag, PredefinedType; the container's GlobalId (IfcRelContainedInSpatialStructure);
  the type's GlobalId; a body on one side only; the box's middle moved by more than the tolerance (by, distance); its
  size changed by more than the tolerance on an axis (old, new); a value of its sets (set, from its type, name; own sets
  then its type's; a real equal to nine digits is the same; a reference the same as a reference);
- the changes in that order, the sets' values in the newer's order then those only the older has.

Writes fixtures/ifc/v1/karsilastirma/*.ifc and fixtures/commands/v1/cad.ifc.compare.json; --check compares. Runs under
IfcOpenShell, offline:

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 \\
        python -I scripts/fixtures/ifc_compare_cases.py [--check]
"""

import copy
import json
import math
import re
import sys
import tempfile
import uuid
from pathlib import Path

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.guid

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ifc_inspect_cases as inspect  # noqa: E402  (the same reading of attributes and sets)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.ifc.compare.json"
FILES = ROOT / "fixtures/ifc/v1/karsilastirma"
REL = "ifc/v1/karsilastirma/"
TOLERANCE = 0.001
WITHIN = 1e-6
SAME = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def guid(seed):
    return ifcopenshell.guid.compress(uuid.uuid5(uuid.NAMESPACE_URL, "kentos:karsilastirma:" + seed).hex)


# The base model: (key, class, name, box or None, container key, extra) — the boxes are metres.
BASE = {
    "W1": {"class": "IfcWall", "name": "D-01", "box": [0.0, 0.0, 0.0, 4.0, 0.2, 3.0], "in": "Z", "tag": "W1",
           "predefined": "STANDARD", "type": "TA",
           "psets": {"Pset_WallCommon": {"FireRating": ("IfcLabel", "REI 60"), "IsExternal": ("IfcBoolean", True)}},
           "qtos": {"Qto_WallBaseQuantities": {"Length": 4.0}}},
    "W2": {"class": "IfcWall", "name": "D-02", "box": [0.0, 3.8, 0.0, 4.0, 4.0, 3.0], "in": "Z", "predefined": "STANDARD",
           "psets": {"Pset_WallCommon": {"FireRating": ("IfcLabel", "REI 60")}}},
    "C1": {"class": "IfcColumn", "name": "K-01", "box": [-0.15, -0.15, 0.0, 0.15, 0.15, 3.0], "in": "Z",
           "predefined": "COLUMN", "psets": {"Pset_ColumnCommon": {"LoadBearing": ("IfcBoolean", True)}}},
    "S1": {"class": "IfcSlab", "name": "DS-01", "box": [0.0, 0.0, 2.8, 4.0, 4.0, 3.0], "in": "K1", "predefined": "FLOOR"},
    "D1": {"class": "IfcDoor", "name": "Kapi 1", "box": None, "in": "Z",
           "psets": {"Pset_DoorCommon": {"FireRating": ("IfcLabel", "EI 30")}}},
}
TYPES = {"TA": {"name": "Duvar tipi A", "psets": {"Pset_WallCommon": {"AcousticRating": ("IfcLabel", "45 dB")}}}}


def written(name, elements, types=TYPES, space_long="Salon", repeat=None):
    """The model with `elements` (BASE's shape), its text with FILE_NAME fixed."""
    f = ifcopenshell.file(schema="IFC4X3_ADD2")
    project = f.createIfcProject(guid("project"), None, "Karşılaştırma")
    m = f.createIfcSIUnit(None, "LENGTHUNIT", None, "METRE")
    project.UnitsInContext = f.createIfcUnitAssignment([m])
    origin = f.createIfcCartesianPoint((0.0, 0.0, 0.0))
    world = f.createIfcAxis2Placement3D(origin, None, None)
    ctx = f.createIfcGeometricRepresentationContext(None, "Model", 3, 1.0e-05, world, None)
    body = f.createIfcGeometricRepresentationSubContext("Body", "Model", None, None, None, None, ctx, None, "MODEL_VIEW",
                                                        None)
    project.RepresentationContexts = [ctx]

    def place(at, rel=None):
        return f.createIfcLocalPlacement(rel, f.createIfcAxis2Placement3D(f.createIfcCartesianPoint(at), None, None))

    site = f.createIfcSite(guid("site"), None, "Arsa", None, None, place((0.0, 0.0, 0.0)), None, None, "ELEMENT")
    building = f.createIfcBuilding(guid("building"), None, "Bina", None, None, site.ObjectPlacement, None, None, "ELEMENT")
    f.createIfcRelAggregates(guid("r-project"), None, None, None, project, [site])
    f.createIfcRelAggregates(guid("r-site"), None, None, None, site, [building])
    storeys = {}
    for key, (sname, z) in {"Z": ("Zemin", 0.0), "K1": ("1. Kat", 3.0)}.items():
        storeys[key] = f.createIfcBuildingStorey(guid("storey:" + key), None, sname, None, None,
                                                 place((0.0, 0.0, z), building.ObjectPlacement), None, sname, "ELEMENT", z)
    f.createIfcRelAggregates(guid("r-building"), None, None, None, building, list(storeys.values()))
    space = f.createIfcSpace(guid("space:Z01"), None, "Z01", None, None, place((0.0, 0.0, 0.0)), None, space_long,
                             "ELEMENT", "INTERNAL", None)
    f.createIfcRelAggregates(guid("r-space"), None, None, None, storeys["Z"], [space])

    def psets_of(owner_key, sets, quantities=None):
        made = []
        for sname, values in sets.items():
            props = [f.createIfcPropertySingleValue(p, None, f.create_entity(kind, v), None) for p, (kind, v) in values.items()]
            made.append(f.createIfcPropertySet(guid(f"pset:{owner_key}:{sname}"), None, sname, None, props))
        for qname, values in (quantities or {}).items():
            qs = [f.createIfcQuantityLength(q, None, None, v, None) for q, v in values.items()]
            made.append(f.createIfcElementQuantity(guid(f"qto:{owner_key}:{qname}"), None, qname, None, None, qs))
        return made

    type_objects = {}
    for key, t in types.items():
        sets = psets_of("type:" + key, t["psets"])
        type_objects[key] = f.createIfcWallType(guid("type:" + key), None, t["name"], None, None, sets, None, None, None,
                                                "STANDARD")
    contained = {"Z": [], "K1": []}
    typed = {}
    for key, e in elements.items():
        shape, at = None, place((0.0, 0.0, 0.0))
        if e.get("box") is not None:
            x0, y0, z0, x1, y1, z1 = e["box"]
            profile = f.createIfcRectangleProfileDef("AREA", None, f.createIfcAxis2Placement2D(
                f.createIfcCartesianPoint((0.0, 0.0)), None), x1 - x0, y1 - y0)
            solid = f.createIfcExtrudedAreaSolid(profile, f.createIfcAxis2Placement3D(
                f.createIfcCartesianPoint((0.0, 0.0, 0.0)), None, None), f.createIfcDirection((0.0, 0.0, 1.0)), z1 - z0)
            shape = f.createIfcProductDefinitionShape(None, None, [f.createIfcShapeRepresentation(
                body, "Body", "SweptSolid", [solid])])
            at = place(((x0 + x1) / 2, (y0 + y1) / 2, z0))
        gid = e.get("guid") or guid("element:" + key)
        cls = e["class"]
        args = [gid, None, e.get("name"), e.get("description"), None, at, shape, e.get("tag")]
        if cls == "IfcDoor":
            args += [None, None, None, None, None]
        else:
            args += [e.get("predefined")]
        el = f.create_entity(cls, *args)
        contained[e["in"]].append(el)
        if e.get("type"):
            typed.setdefault(e["type"], []).append(el)
        sets = psets_of(key, e.get("psets", {}), e.get("qtos"))
        for k, s in enumerate(sets):
            f.createIfcRelDefinesByProperties(guid(f"rdp:{key}:{k}"), None, None, None, [el], s)
    for key, els in typed.items():
        f.createIfcRelDefinesByType(guid("rdt:" + key), None, None, None, els, type_objects[key])
    for key, els in contained.items():
        if els:
            f.createIfcRelContainedInSpatialStructure(guid("rc:" + key), None, None, None, els, storeys[key])
    if repeat:
        # Another wall said with an existing GlobalId.
        x = f.createIfcWall(repeat, None, "D-99", None, None, place((10.0, 0.0, 0.0)), None, None, "STANDARD")
        f.createIfcRelContainedInSpatialStructure(guid("rc:repeat"), None, None, None, [x], storeys["Z"])
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "x.ifc"
        f.write(str(p))
        text = p.read_text(encoding="utf-8")
    fixed = (f"FILE_NAME('{name}','2026-11-17T12:00:00',(''),(''),'IfcOpenShell {ifcopenshell.version}',"
             f"'IfcOpenShell','');")
    return re.sub(r"FILE_NAME\(.*?\);", lambda _: fixed, text, count=1, flags=re.S)


# Reading, by IfcOpenShell.
def objects_of(f):
    return sorted(f.by_type("IfcObjectDefinition"), key=lambda o: o.id())


def box_of(o):
    if not getattr(o, "Representation", None):
        return None
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    try:
        v = ifcopenshell.geom.create_shape(settings, o).geometry.verts
    except Exception:  # noqa: BLE001  (no body made: none)
        return None
    if not v:
        return None
    xs, ys, zs = v[0::3], v[1::3], v[2::3]
    return [min(xs), min(ys), min(zs), max(xs), max(ys), max(zs)]


def container_of(o):
    for r in getattr(o, "ContainedInStructure", None) or ():
        return r.RelatingStructure
    return None


def type_of(f, o):
    for r in sorted(f.by_type("IfcRelDefinesByType"), key=lambda r: r.id()):
        if o in (r.RelatedObjects or ()):
            return r.RelatingType
    return None


def named(o):
    out = {"globalId": o.GlobalId, "class": o.is_a()}
    if o.Name is not None:
        out["name"] = o.Name
    return out


def values_of(f, o):
    d = inspect.details(f, o)
    out, seen = [], set()
    for s in d["sets"]:
        if s["kind"] == "other":
            key = (s["name"], s["fromType"], "")
            if key not in seen:
                seen.add(key)
                out.append((key, {"kind": "empty"}))
            continue
        for v in s["values"]:
            key = (s["name"], s["fromType"], v["name"])
            if key not in seen:
                seen.add(key)
                out.append((key, v["value"]))
    return d, out


def same_value(a, b):
    if a["kind"] == "number" and b["kind"] == "number":
        x, y = a["number"], b["number"]
        return a.get("measure") == b.get("measure") and (x == y or abs(x - y) <= 1e-9 * max(abs(x), abs(y), 1.0))
    if a["kind"] == "reference" and b["kind"] == "reference":
        return True
    return a == b


def compared(old_text, new_text, tolerance):
    with tempfile.TemporaryDirectory() as d:
        (Path(d) / "a.ifc").write_text(old_text, encoding="utf-8")
        (Path(d) / "b.ifc").write_text(new_text, encoding="utf-8")
        fa, fb = ifcopenshell.open(str(Path(d) / "a.ifc")), ifcopenshell.open(str(Path(d) / "b.ifc"))
        oa, ob = objects_of(fa), objects_of(fb)
        count = lambda objs: {g: sum(1 for o in objs if o.GlobalId == g) for g in {o.GlobalId for o in objs}}  # noqa: E731
        ca, cb = count(oa), count(ob)
        twice_a = {g for g, n in ca.items() if n > 1}
        twice_b = {g for g, n in cb.items() if n > 1}
        by_a = {o.GlobalId: o for o in oa if o.GlobalId not in twice_a}
        by_b = {o.GlobalId for o in ob}
        added, changed, unchanged = [], [], 0
        for n in ob:
            g = n.GlobalId
            if g in twice_b:
                continue
            if g not in by_a:
                added.append(named(n))
                continue
            o = by_a[g]
            changes = []
            if o.is_a() != n.is_a():
                changes.append({"kind": "class", "old": o.is_a(), "new": n.is_a()})
            da, va = values_of(fa, o)
            db, vb = values_of(fb, n)
            for attr, key in [("Name", "name"), ("Description", "description"), ("ObjectType", "objectType"),
                              ("LongName", "longName"), ("Tag", "tag"), ("PredefinedType", "predefinedType")]:
                if da[key] != db[key]:
                    c = {"kind": "attribute", "name": attr}
                    if da[key] is not None:
                        c["old"] = da[key]
                    if db[key] is not None:
                        c["new"] = db[key]
                    changes.append(c)
            for kind, fn in [("container", lambda f, x: container_of(x)), ("type", type_of)]:
                xa, xb = fn(fa, o), fn(fb, n)
                if (xa.GlobalId if xa else None) != (xb.GlobalId if xb else None):
                    c = {"kind": kind}
                    if xa is not None:
                        c["old"] = named(xa)
                    if xb is not None:
                        c["new"] = named(xb)
                    changes.append(c)
            ba, bb = box_of(o), box_of(n)
            if ba is not None and bb is not None:
                by = [(bb[k] + bb[k + 3] - ba[k] - ba[k + 3]) / 2 for k in range(3)]
                dist = math.sqrt(sum(x * x for x in by))
                if dist > tolerance:
                    changes.append({"kind": "moved", "by": by, "distance": dist})
                sa = [ba[k + 3] - ba[k] for k in range(3)]
                sb = [bb[k + 3] - bb[k] for k in range(3)]
                if any(abs(sa[k] - sb[k]) > tolerance for k in range(3)):
                    changes.append({"kind": "resized", "old": sa, "new": sb})
            elif (ba is None) != (bb is None):
                changes.append({"kind": "body", "old": ba is not None, "new": bb is not None})
            old_map = dict(va)
            new_keys = {k for k, _ in vb}
            for key, value in vb:
                was = old_map.get(key)
                if was is not None and same_value(was, value):
                    continue
                c = {"kind": "property", "set": key[0], "fromType": key[1], "name": key[2]}
                if was is not None:
                    c["old"] = was
                c["new"] = value
                changes.append(c)
            for key, value in va:
                if key not in new_keys:
                    changes.append({"kind": "property", "set": key[0], "fromType": key[1], "name": key[2], "old": value})
            if changes:
                row = {"globalId": g, "class": n.is_a(), "changes": changes}
                if n.Name is not None:
                    row["name"] = n.Name
                changed.append(row)
            else:
                unchanged += 1
        removed = [named(o) for o in oa if o.GlobalId not in twice_a and o.GlobalId not in by_b]
        repeated = sorted([("older", g) for g in twice_a] + [("newer", g) for g in twice_b], key=lambda x: (x[0] == "newer", x[1]))
        return {"added": added, "removed": removed, "changed": changed, "unchanged": unchanged,
                "repeated": [{"file": w, "globalId": g} for w, g in repeated],
                "schemas": (fa.schema_identifier if hasattr(fa, "schema_identifier") else fa.schema,
                            fb.schema_identifier if hasattr(fb, "schema_identifier") else fb.schema),
                "objects": (len(oa), len(ob))}


def main():
    check = "--check" in sys.argv
    variants = {}

    def variant(name, mutate=None, **kw):
        els = copy.deepcopy(BASE)
        types = copy.deepcopy(TYPES)
        if mutate:
            mutate(els, types)
        variants[name] = written(name, els, types, **kw)

    variant("taban.ifc")
    variant("ayni.ifc")
    variant("tasinan.ifc", lambda e, t: e["W1"].update(box=[0.0, 0.3, 0.0, 4.0, 0.5, 3.0]))
    variant("az-tasinan.ifc", lambda e, t: e["W1"].update(box=[0.0, 0.0005, 0.0, 4.0, 0.2005, 3.0]))
    variant("uzayan.ifc", lambda e, t: e["W2"].update(box=[0.0, 3.8, 0.0, 5.0, 4.0, 3.0]))
    variant("sinif.ifc", lambda e, t: e["C1"].update({"class": "IfcMember", "predefined": "POST"}))
    variant("oznitelik.ifc", lambda e, t: e["W1"].update(name="D-01a", description="Taşıyıcı değil", tag=None),
            space_long="Oturma odası")
    variant("kat.ifc", lambda e, t: e["W2"].update({"in": "K1"}))
    variant("tip.ifc", lambda e, t: (e["W2"].update(type="TA"), e["W1"].pop("type")))

    def props(e, t):
        e["W1"]["psets"]["Pset_WallCommon"]["FireRating"] = ("IfcLabel", "REI 90")
        e["W1"]["psets"]["Pset_WallCommon"]["IsExternal"] = ("IfcBoolean", False)
        e["W1"]["psets"]["Pset_WallCommon"]["LoadBearing"] = ("IfcBoolean", True)
        e["W1"]["qtos"]["Qto_WallBaseQuantities"]["Length"] = 4.0000000001
        e["D1"].pop("psets")
    variant("ozellik.ifc", props)
    variant("tip-ozellik.ifc", lambda e, t: t["TA"]["psets"]["Pset_WallCommon"].update(AcousticRating=("IfcLabel", "50 dB")))

    def added_removed(e, t):
        e.pop("S1")
        e["W3"] = {"class": "IfcWall", "name": "D-03", "box": [4.0, 0.0, 0.0, 4.2, 4.0, 3.0], "in": "Z",
                   "predefined": "STANDARD"}
    variant("eklenen-silinen.ifc", added_removed)
    variant("govde.ifc", lambda e, t: e["D1"].update(box=[1.0, 0.0, 0.0, 1.9, 0.2, 2.1]))
    variant("tekrar.ifc", repeat=guid("element:W2"))

    base = variants["taban.ifc"]
    cases = []

    def path(name):
        return "$fixture:" + REL + name

    def ok(title, newer, tolerance=None, older="taban.ifc", op="execute", warnings=()):
        c = compared(variants[older], variants[newer], tolerance or TOLERANCE)
        out = {"older": {"path": path(older), "schema": c["schemas"][0], "objects": c["objects"][0]},
               "newer": {"path": path(newer), "schema": c["schemas"][1], "objects": c["objects"][1]},
               "added": c["added"], "removed": c["removed"], "changed": c["changed"], "unchanged": c["unchanged"],
               "repeated": c["repeated"], "tolerance": tolerance or TOLERANCE}
        inp = {"older": path(older), "newer": path(newer)}
        if tolerance is not None:
            inp["tolerance"] = tolerance
        cases.append({"name": title, "steps": [{"op": op, "input": inp, "within": WITHIN, "expect": SAME,
                                                 "result": {"status": "completed", "output": out,
                                                            "warnings": list(warnings)}}]})
        return c

    def refused(title, inp, code, at):
        cases.append({"name": title, "steps": [{"op": "execute", "input": inp, "expect": SAME, "result": {
            "status": "failed", "error": {"code": code, "message": "$any", "path": at}}}]})

    ok("Aynı model: değişiklik yok", "ayni.ifc")
    ok("Dosya kendisiyle: değişiklik yok", "taban.ifc")
    ok("Taşınan duvar: kutusunun ortası 0,30 m kaydı", "tasinan.ifc")
    ok("Toleranstan az kayma değişiklik değildir", "az-tasinan.ifc")
    ok("0,1 mm toleransta yarım milimetre kayma değişikliktir", "az-tasinan.ifc", tolerance=0.0001)
    ok("Uzayan duvar: ortası kaydı, boyu değişti", "uzayan.ifc")
    ok("0,5 m toleransta 0,5 m'lik değişiklik görülmez", "uzayan.ifc", tolerance=0.5)
    ok("Sınıfı değişen kolon", "sinif.ifc")
    ok("Adı, açıklaması, etiketi ve mahalin uzun adı değişti", "oznitelik.ifc")
    ok("Başka kata alınan duvar", "kat.ifc")
    ok("Tipi değişen duvarlar", "tip.ifc")
    ok("Yangın sınıfı, dış duvar, eklenen ve silinen özellikler; dokuz basamağa eşit miktar aynı", "ozellik.ifc")
    ok("Tipin özelliği: tipi ve onu taşıyan duvar", "tip-ozellik.ifc")
    ok("Eklenen duvar, silinen döşeme", "eklenen-silinen.ifc")
    ok("Gövdesi olmayan kapıya gövde gelir", "govde.ifc")
    ok("Ters yönde: eklenen silinen, silinen eklenen olur", "taban.ifc", older="eklenen-silinen.ifc")
    ok("Önizleme aynı sonucu verir", "ozellik.ifc", op="plan")
    ok("Bir GlobalId iki nesnede: karşılaştırılmaz, uyarılır", "tekrar.ifc",
       warnings=[{"code": "ifc_repeated_global_id", "message": "$any"}])
    t = path("taban.ifc")
    refused("Eski dosyanın yolu boş", {"older": "", "newer": t}, "invalid_file_path", "older")
    refused("Eski dosyanın yolu göreli", {"older": "taban.ifc", "newer": t}, "invalid_file_path", "older")
    refused("Eski dosya .ifc değil", {"older": "$fixture:" + REL + "taban.txt", "newer": t}, "invalid_file_path", "older")
    refused("Yeni dosya yok", {"older": t, "newer": "$fixture:" + REL + "yok.ifc"}, "invalid_file_path", "newer")
    refused("Negatif tolerans", {"older": t, "newer": t, "tolerance": -0.001}, "invalid_tolerance", "tolerance")
    refused("0,1 mm'den küçük tolerans", {"older": t, "newer": t, "tolerance": 0.00005}, "invalid_tolerance", "tolerance")
    refused("0,5 m'den büyük tolerans", {"older": t, "newer": t, "tolerance": 0.6}, "invalid_tolerance", "tolerance")
    refused("Yollar toleranstan önce", {"older": t, "newer": "", "tolerance": 0.6}, "invalid_file_path", "newer")
    refused("IFC olmayan eski dosya", {"older": "$fixture:arch/v1/ifc/var.ifc", "newer": t}, "ifc_unreadable", "older")
    refused("IFC olmayan yeni dosya", {"older": t, "newer": "$fixture:arch/v1/ifc/var.ifc"}, "ifc_unreadable", "newer")

    setup = copy.deepcopy(inspect.SETUP)
    setup["name"] = "IFC karşılaştırması"
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.ifc.compare", "commandVersion": 1,
           "title": "IFC revizyonlarını karşılaştır: GlobalId ile eşlenen nesnelerin eklenmesi, silinmesi ve değişiklikleri",
           "note": "ADR 8103. Dosyaları IfcOpenShell yazar; beklenenler scripts/fixtures/ifc_compare_cases.py'nin ADR'den "
                   "yazılmış kurallarıyla IfcOpenShell'in okumasından, KentOS kodu yok.",
           "setup": setup, "cases": cases}
    outputs = {OUT: json.dumps(doc, ensure_ascii=False, indent=2) + "\n"}
    for name, text in variants.items():
        outputs[FILES / name] = text
    outputs[FILES / "taban.txt"] = base
    stale = []
    for p, text in outputs.items():
        if check:
            if not p.exists() or p.read_text(encoding="utf-8") != text:
                stale.append(p.name)
        else:
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text, encoding="utf-8")
    if stale:
        print("güncel değil: " + ", ".join(stale), file=sys.stderr)
        return 1
    print(("güncel: " if check else "yazıldı: ") + f"{len(cases)} durum, {len(outputs)} dosya")
    return 0


if __name__ == "__main__":
    sys.exit(main())
