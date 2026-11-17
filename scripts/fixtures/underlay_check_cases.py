#!/usr/bin/env python3
"""cad.underlay.check v1 (“Altlık modeli denetle”, docs/adr/8102): the shared cases, written from the ADR without
KentOS code.

The drawing: TUREF / TM36 (EPSG:5256), its heights Antalya height (EPSG:5775), ±0.00 at 1285.4 m; levels Zemin kat (±0.00) and 1. kat
(3.00), each 3.00 m floor to floor; on Zemin kat two 30/30 columns at (0, 0) and (6, 0) and a 20 cm wall from (0, 5) to
(6, 5), justified on its axis; grids A (x 0), B (x 6), 1 (y 0), 2 (y 5). Its model's bodies, by the rules that make
them (no slab stands over either): a column's its section about its point, from its level's floor up its own height
or else its level's (docs/adr/8070 §28); a wall's its axis widened by half its width to each side, from its level's
floor up its level's height (docs/adr/8040 §1). Their names (EK-5 §5.5, docs/adr/8047): “MM-KLN-Kolon” (a column's type
unsaid: Kolon), “MM-DVR” (a wall with no style: no description).

The files, written here by IfcOpenShell (not by KentOS) as another discipline's model: millimetres, the same system,
±0.00 at 1285400 mm (IfcMapConversion H 1285.4, Scale 0.001), storeys Zemin (0) and 1. Kat (3000), grids where the
drawing's are, each element an extruded rectangle (its box):

- denetim-hizali.ifc: the two columns where the drawing's stand, and a beam between them (x 0.15 to 5.85, z 2.6 to
  3.0) whose ends touch the columns' faces.
- denetim-kaymis.ifc: the column on A-1 moved 0.20 m east, 1. Kat said at 3150, B at x 6.10, 2 from (-2, 5) to (8, 5.1);
  the beam between the columns; a beam through the wall (x 2.9 to 3.1, y 4.5 to 5.5, z 2.5 to 2.9); a member inside it
  (x 1 to 1.5, y 4.95 to 5.05, z 1 to 1.2); a slab sunk 2 cm on the columns (x -0.5 to 6.5, y -0.5 to 0.5, z 2.98 to
  3.18).
- denetim-datum.ifc: denetim-hizali.ifc with its heights on ITRF96.
- denetim-sistem.ifc: denetim-hizali.ifc on TUREF / TM33 (EPSG:5255).
- denetim-bos.ifc: no element and no grid; storeys Zemin (0) and Çatı (no elevation).

The rules (ADR 8102 §2):
- Refusals, in order: `id` a lowercase hyphenated UUID (invalid_uid at `id`) the drawing holds
  (model_underlay_not_found at `id`); `tolerance` 0.0001 to 0.5 m (invalid_tolerance at `tolerance`); the file there
  (file_not_found at `id`), its SHA-256 the drawing's (model_underlay_changed at `id`), read as IFC
  (model_underlay_unreadable at `id`), placed as held (model_underlay_not_placed at `id`: another system, another height
  datum, a project with no system; at the origin always placed). Placed by its map conversion while the project (or
  the file) does not say its height datum: the heights are taken as the same, and the warning underlay_datum_unsaid
  (at `id`) says so.
- Levels: every pair of a project level and a file storey with an elevation, by their difference, the smaller first
  (then the level's order, then the storey's); a level or storey already taken is passed. The pairs in the project's
  level order, then the project's unpaired levels, then the file's unpaired storeys, then its storeys with no elevation
  (no elevation, no pair). Difference: the storey's less the level's.
- Axes: by label (sorted); both: the distance of the file axis's middle from the project's line and the turn between
  them in degrees (0 to 90).
- Elements: a file element and a drawing one of the same class whose boxes stand within the tolerance on every side
  are matched, each once, the file's in its order taking the drawing's first free one by persistent id. The file's other elements
  against every drawing element: their boxes overlapping by more than the tolerance on every axis clash (for boxes,
  the bodies take the same room); `inside` when one box holds the other. Clashes in the file's order, each file object's
  by the drawing's persistent id. Unmatched: of the file's classes, the drawing's elements neither matched nor clashing
  (by persistent id), then the file's (in its order).
- A check reads only: the drawing does not change, no undo step.

Runs under IfcOpenShell, offline:

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 \\
        python -I scripts/fixtures/underlay_check_cases.py [--check]
"""
import copy
import hashlib
import json
import math
import re
import sys
import tempfile
import uuid
from pathlib import Path

import ifcopenshell
import ifcopenshell.guid

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.underlay.check.json"
FILES = ROOT / "fixtures/ifc/v1/statik"
REL = "ifc/v1/statik/"
COMMAND = "cad.underlay.check"
TOLERANCE = 0.001
LEAST, MOST = 0.0001, 0.5
WITHIN = 1e-6

LEVELS = [("zemin", "Zemin kat", 0.0, 3.0), ("kat1", "1. kat", 3.0, 3.0)]
GRIDS = [("A", (0.0, -2.0), (0.0, 7.0)), ("B", (6.0, -2.0), (6.0, 7.0)),
         ("1", (-2.0, 0.0), (8.0, 0.0)), ("2", (-2.0, 5.0), (8.0, 5.0))]


def U(n):
    return f"0192f5a1-8102-7000-8000-{n:012x}"


# The drawing's objects: (persistent id, IFC class, name, box) for the bodies; the entities follow.
C1, C2, WALL = U(1), U(2), U(3)
A, B = U(0xa), U(0xb)


def column_box(x, y, w, d, floor, height):
    return [x - w / 2, y - d / 2, floor, x + w / 2, y + d / 2, floor + height]


def own(c1_height=None):
    """The drawing's bodies: its two columns (the first its own height when given) and its wall."""
    return [
        (C1, "IfcColumn", "MM-KLN-Kolon", column_box(0.0, 0.0, 0.3, 0.3, 0.0, c1_height or 3.0)),
        (C2, "IfcColumn", "MM-KLN-Kolon", column_box(6.0, 0.0, 0.3, 0.3, 0.0, 3.0)),
        (WALL, "IfcWall", "MM-DVR", [0.0, 4.9, 0.0, 6.0, 5.1, 3.0]),
    ]


def entities(c1_height=None):
    col = lambda i, x, y: {"kind": "column", "id": i, "layerId": "0", "attrs": {}, "at": {"x": x, "y": y},
                           "width": 0.3, "depth": 0.3, "arch": {"level": "zemin"}}
    first = col(1, 0.0, 0.0)
    if c1_height is not None:
        first["height"] = c1_height
    out = [first, col(2, 6.0, 0.0),
           {"kind": "wall", "id": 3, "layerId": "0", "attrs": {}, "pts": [{"x": 0.0, "y": 5.0}, {"x": 6.0, "y": 5.0}],
            "width": 0.2, "justify": "center", "arch": {"level": "zemin"}}]
    for k, (label, a, b) in enumerate(GRIDS):
        out.append({"kind": "grid", "id": 4 + k, "layerId": "0", "attrs": {},
                    "pts": [{"x": a[0], "y": a[1]}, {"x": b[0], "y": b[1]}], "name": label})
    return out


def setup(underlays, srid=5256, c1_height=None, heights=True):
    arch = {"site": {"datum": 1285.4},
            "levels": [{"id": i, "name": n, "elevation": e, "height": h} for i, n, e, h in LEVELS]}
    if underlays:
        arch["underlays"] = underlays
    settings = {"srid": srid, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 50.0, "workspace": "cad", "arch": arch}
    if heights:
        # Antalya height (EPSG:5775): the project's IFC files say it as their height datum.
        settings["heights"] = {"kind": "orthometric", "verticalSrid": 5775}
    return {"format": "kentos.document", "version": 2, "name": "Altlık denetimi", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0},
            "layers": [{"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True,
                        "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
            "activeLayer": "0", "entities": entities(c1_height), "uids": [U(1), U(2), U(3), U(4), U(5), U(6), U(7)],
            "styles": {"items": [], "categories": []}}


# The files: what each holds, in the file's order.
def guid(seed):
    return ifcopenshell.guid.compress(uuid.uuid5(uuid.NAMESPACE_URL, "kentos:denetim:" + seed).hex)


HIZALI = {
    "storeys": [("Zemin", 0.0), ("1. Kat", 3.0)],
    "grids": GRIDS,
    "elements": [("IfcColumn", "S-C1", [-0.15, -0.15, 0.0, 0.15, 0.15, 3.0]),
                 ("IfcColumn", "S-C2", [5.85, -0.15, 0.0, 6.15, 0.15, 3.0]),
                 ("IfcBeam", "S-K1", [0.15, -0.15, 2.6, 5.85, 0.15, 3.0])],
}
KAYMIS = {
    "storeys": [("Zemin", 0.0), ("1. Kat", 3.15)],
    "grids": [("A", (0.0, -2.0), (0.0, 7.0)), ("B", (6.1, -2.0), (6.1, 7.0)),
              ("1", (-2.0, 0.0), (8.0, 0.0)), ("2", (-2.0, 5.0), (8.0, 5.1))],
    "elements": [("IfcColumn", "S-C1", [0.05, -0.15, 0.0, 0.35, 0.15, 3.0]),
                 ("IfcColumn", "S-C2", [5.85, -0.15, 0.0, 6.15, 0.15, 3.0]),
                 ("IfcBeam", "S-K1", [0.15, -0.15, 2.6, 5.85, 0.15, 3.0]),
                 ("IfcBeam", "S-K2", [2.9, 4.5, 2.5, 3.1, 5.5, 2.9]),
                 ("IfcMember", "S-E1", [1.0, 4.95, 1.0, 1.5, 5.05, 1.2]),
                 ("IfcSlab", "S-D1", [-0.5, -0.5, 2.98, 6.5, 0.5, 3.18])],
}
BOS = {"storeys": [("Zemin", 0.0), ("Çatı", None)], "grids": [], "elements": []}
TM36 = ("EPSG:5256", "TUREF / TM36 (EPSG:5256)", "36")
TM33 = ("EPSG:5255", "TUREF / TM33 (EPSG:5255)", "33")
ANTALYA = "Antalya height (EPSG:5775)"


def written(name, held, system=TM36, datum=ANTALYA):
    """The file: its text (FILE_NAME fixed) and the GlobalIds of its elements in its order."""
    f = ifcopenshell.file(schema="IFC4X3_ADD2")
    g = lambda s: guid(name + ":" + s)
    project = f.createIfcProject(g("project"), None, "Altlık denetimi — statik")
    units = [f.createIfcSIUnit(None, "LENGTHUNIT", "MILLI", "METRE"),
             f.createIfcSIUnit(None, "AREAUNIT", None, "SQUARE_METRE"),
             f.createIfcSIUnit(None, "VOLUMEUNIT", None, "CUBIC_METRE"),
             f.createIfcSIUnit(None, "PLANEANGLEUNIT", None, "RADIAN")]
    project.UnitsInContext = f.createIfcUnitAssignment(units)
    world = f.createIfcAxis2Placement3D(f.createIfcCartesianPoint((0.0, 0.0, 0.0)), None, None)
    ctx = f.createIfcGeometricRepresentationContext(None, "Model", 3, 1.0e-05, world, None)
    body = f.createIfcGeometricRepresentationSubContext("Body", "Model", None, None, None, None, ctx, None,
                                                        "MODEL_VIEW", None)
    project.RepresentationContexts = [ctx]
    crs = f.createIfcProjectedCRS(system[0], system[1], "TUREF", datum, "Transverse Mercator", system[2], None)
    f.createIfcMapConversion(ctx, crs, 0.0, 0.0, 1285.4, 1.0, 0.0, 0.001)

    def place(at, relative=None):
        return f.createIfcLocalPlacement(relative, f.createIfcAxis2Placement3D(f.createIfcCartesianPoint(at), None, None))

    site = f.createIfcSite(g("site"), None, "Arsa", None, None, place((0.0, 0.0, 0.0)), None, None, "ELEMENT")
    building = f.createIfcBuilding(g("building"), None, "Bina", None, None, site.ObjectPlacement, None, None, "ELEMENT")
    f.createIfcRelAggregates(g("project-site"), None, None, None, project, [site])
    f.createIfcRelAggregates(g("site-building"), None, None, None, site, [building])
    storeys = []
    for sname, elevation in held["storeys"]:
        z = (elevation or 0.0) * 1000.0
        s = f.createIfcBuildingStorey(g("storey:" + sname), None, sname, None, None,
                                      place((0.0, 0.0, z), building.ObjectPlacement), None, sname, "ELEMENT",
                                      None if elevation is None else elevation * 1000.0)
        storeys.append(s)
    f.createIfcRelAggregates(g("building-storeys"), None, None, None, building, storeys)
    contained = []
    if held["grids"]:
        def axis(tag, a, b):
            line = f.createIfcPolyline([f.createIfcCartesianPoint((a[0] * 1000.0, a[1] * 1000.0)),
                                        f.createIfcCartesianPoint((b[0] * 1000.0, b[1] * 1000.0))])
            return f.createIfcGridAxis(tag, line, True)
        u = [axis(t, a, b) for t, a, b in held["grids"] if t.isalpha()]
        v = [axis(t, a, b) for t, a, b in held["grids"] if not t.isalpha()]
        contained.append(f.createIfcGrid(g("grid"), None, "Akslar", None, None,
                                         place((0.0, 0.0, 0.0), storeys[0].ObjectPlacement), None, u, v, None, None))
    ids = []
    for cls, ename, (x0, y0, z0, x1, y1, z1) in held["elements"]:
        profile = f.createIfcRectangleProfileDef(
            "AREA", None, f.createIfcAxis2Placement2D(f.createIfcCartesianPoint((0.0, 0.0)), None),
            round((x1 - x0) * 1000.0, 6), round((y1 - y0) * 1000.0, 6))
        solid = f.createIfcExtrudedAreaSolid(
            profile, f.createIfcAxis2Placement3D(f.createIfcCartesianPoint((0.0, 0.0, 0.0)), None, None),
            f.createIfcDirection((0.0, 0.0, 1.0)), round((z1 - z0) * 1000.0, 6))
        rep = f.createIfcShapeRepresentation(body, "Body", "SweptSolid", [solid])
        shape = f.createIfcProductDefinitionShape(None, None, [rep])
        at = place((round((x0 + x1) / 2 * 1000.0, 6), round((y0 + y1) / 2 * 1000.0, 6), round(z0 * 1000.0, 6)))
        gid = g("element:" + ename)
        contained.append(getattr(f, "create" + cls)(gid, None, ename, None, None, at, shape, None, None))
        ids.append(gid)
    if contained:
        f.createIfcRelContainedInSpatialStructure(g("contains"), None, None, None, contained, storeys[0])
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "x.ifc"
        f.write(str(p))
        text = p.read_text(encoding="utf-8")
    fixed = (f"FILE_NAME('{name}','2026-11-17T12:00:00',(''),(''),'IfcOpenShell {ifcopenshell.version}',"
             f"'IfcOpenShell','');")
    return re.sub(r"FILE_NAME\(.*?\);", lambda _: fixed, text, count=1, flags=re.S), ids


# The rules, from the ADR.
def pair_levels(levels, storeys):
    """levels: [(name, elevation)]; storeys: [(name, elevation or None)] in the file's order."""
    said = [(j, s) for j, s in enumerate(storeys) if s[1] is not None]
    pairs = sorted(((abs(s[1] - l[1]), i, j) for i, l in enumerate(levels) for j, s in said))
    lt, st, out = set(), set(), []
    for _, i, j in pairs:
        if i not in lt and j not in st:
            lt.add(i)
            st.add(j)
            out.append((i, j))
    out.sort()
    rows = []
    for i, j in out:
        rows.append({"level": levels[i][0], "storey": storeys[j][0], "project": levels[i][1], "file": storeys[j][1],
                     "difference": storeys[j][1] - levels[i][1]})
    rows += [{"level": l[0], "project": l[1]} for i, l in enumerate(levels) if i not in lt]
    rows += [{"storey": s[0], "file": s[1]} for j, s in said if j not in st]
    rows += [{"storey": s[0]} for s in storeys if s[1] is None]
    return rows


def axis_rows(project, file):
    out = []
    for label in sorted({g[0] for g in project} | {g[0] for g in file}):
        p = next((g for g in project if g[0] == label), None)
        f = next((g for g in file if g[0] == label), None)
        row = {"label": label, "inProject": p is not None, "inFile": f is not None}
        if p and f:
            (ax, ay), (bx, by) = p[1], p[2]
            (cx, cy), (dx, dy) = f[1], f[2]
            da, db = (bx - ax, by - ay), (dx - cx, dy - cy)
            la, lb = math.hypot(*da), math.hypot(*db)
            mid = ((cx + dx) / 2, (cy + dy) / 2)
            row["offset"] = abs((mid[0] - ax) * da[1] - (mid[1] - ay) * da[0]) / la
            sin = min(abs(da[0] * db[1] - da[1] * db[0]) / (la * lb), 1.0)
            row["turn"] = math.degrees(math.asin(sin))
        out.append(row)
    return out


def same_box(a, b, t):
    return all(abs(a[k] - b[k]) <= t for k in range(6))


def overlap(a, b, t):
    o = [max(a[0], b[0]), max(a[1], b[1]), max(a[2], b[2]), min(a[3], b[3]), min(a[4], b[4]), min(a[5], b[5])]
    return o if all(o[k + 3] - o[k] > t for k in range(3)) else None


def within(x, y):
    return all(x[k] >= y[k] and x[k + 3] <= y[k + 3] for k in range(3))


def checked(id_, held, ids, tolerance=TOLERANCE, c1_height=None):
    bodies = own(c1_height)
    levels = [(n, e) for _, n, e, _ in LEVELS]
    taken, matched_file = set(), set()
    for j, (cls, _, box) in enumerate(held["elements"]):
        for i, (_, ocls, _, obox) in sorted(enumerate(bodies), key=lambda x: x[1][0]):
            if i not in taken and ocls.lower() == cls.lower() and same_box(obox, box, max(tolerance, 1e-6)):
                taken.add(i)
                matched_file.add(j)
                break
    clashes = []
    clashing = set()
    for j, (cls, name, box) in enumerate(held["elements"]):
        if j in matched_file:
            continue
        for uid, ocls, oname, obox in sorted(bodies):
            o = overlap(obox, box, tolerance)
            if o is None:
                continue
            clashes.append({"uid": uid, "class": ocls, "name": oname, "object": ids[j], "objectClass": cls,
                            "objectName": name, "overlap": o, "inside": within(box, obox) or within(obox, box)})
            clashing.add(j)
    classes = {e[0].lower() for e in held["elements"]}
    own_clashing = {c["uid"] for c in clashes}
    unmatched = [{"uid": uid, "class": ocls, "name": oname}
                 for i, (uid, ocls, oname, _) in sorted(enumerate(bodies), key=lambda x: x[1][0])
                 if i not in taken and uid not in own_clashing and ocls.lower() in classes]
    unmatched += [{"object": ids[j], "class": cls, "name": name}
                  for j, (cls, name, _) in enumerate(held["elements"]) if j not in matched_file and j not in clashing]
    return {"id": id_, "levels": pair_levels(levels, held["storeys"]),
            "axes": axis_rows(GRIDS, held["grids"]), "matched": len(matched_file), "clashes": clashes,
            "unmatched": unmatched, "tolerance": tolerance, "revision": "$current"}


def main():
    check = "--check" in sys.argv
    texts, ids = {}, {}
    for name, held, system, datum in [("denetim-hizali.ifc", HIZALI, TM36, ANTALYA),
                                      ("denetim-kaymis.ifc", KAYMIS, TM36, ANTALYA),
                                      ("denetim-datum.ifc", HIZALI, TM36, "ITRF96"),
                                      ("denetim-sistem.ifc", HIZALI, TM33, ANTALYA),
                                      ("denetim-bos.ifc", BOS, TM36, ANTALYA)]:
        texts[name], ids[name] = written(name, held, system, datum)

    def entry(id_, name, placement=None, shown=True, sha=None, file=None):
        data = texts[name].encode("utf-8") if name in texts else (ROOT / "fixtures" / file).read_bytes()
        u = {"id": id_, "name": name, "file": "$fixture:" + (file or REL + name),
             "sha256": sha or hashlib.sha256(data).hexdigest(), "bytes": len(data),
             "schema": "IFC4X3_ADD2" if name in texts else "IFC4", "discipline": "Statik"}
        if placement:
            u["placement"] = placement
        if not shown:
            u["shown"] = False
        return u

    same = {"dirty": False, "canUndo": False, "revision": "same"}

    def done(name, underlays, input_, output, op="execute", warnings=(), **over):
        s = setup(underlays, **over)
        return {"name": name, "setup": s, "steps": [{"op": op, "input": input_, "within": WITHIN,
                                                    "result": {"status": "completed", "output": output,
                                                               "warnings": list(warnings)}, "expect": same}]}

    def refused(name, underlays, input_, code, path, **over):
        s = setup(underlays, **over)
        return {"name": name, "setup": s, "steps": [{"op": "execute", "input": input_,
                                                    "result": {"status": "failed",
                                                               "error": {"code": code, "message": "$any", "path": path}},
                                                    "expect": same}]}

    hz, ky, bs = (entry(A, "denetim-hizali.ifc"), entry(A, "denetim-kaymis.ifc"), entry(A, "denetim-bos.ifc"))
    kh = ids["denetim-hizali.ifc"]
    kk = ids["denetim-kaymis.ifc"]
    cases = [
        done("Hizalı statik model: kolonlar eşleşir, kolonlara değen kiriş çakışmaz", [hz], {"id": A},
             checked(A, HIZALI, kh)),
        done("Önizleme aynı sonucu verir", [hz], {"id": A}, checked(A, HIZALI, kh), op="plan"),
        done("Kaydırılan kolon, yükseltilen kat, kayan ve dönen aks, duvarı delen kiriş, duvarın içindeki eleman, "
             "kolonlara 2 cm gömülen döşeme", [ky], {"id": A}, checked(A, KAYMIS, kk)),
        done("Tolerans 5 cm: 2 cm'lik gömülme çakışma değildir", [ky], {"id": A, "tolerance": 0.05},
             checked(A, KAYMIS, kk, 0.05)),
        done("Tolerans 25 cm: 20 cm kaydırılan kolon aynı kolondur", [ky], {"id": A, "tolerance": 0.25},
             checked(A, KAYMIS, kk, 0.25)),
        done("En büyük tolerans 0,5 m", [ky], {"id": A, "tolerance": MOST}, checked(A, KAYMIS, kk, MOST)),
        done("En küçük tolerans 0,1 mm", [ky], {"id": A, "tolerance": LEAST}, checked(A, KAYMIS, kk, LEAST)),
        done("Elemansız model: katlar eşlenir, kotu yazılmamış kat kotsuz listelenir, akslar yalnız projede", [bs],
             {"id": A}, checked(A, BOS, [])),
        done("Kendi yüksekliği 2,50 m olan kolon dosyanınkiyle eşleşmez, içinde kalır", [hz], {"id": A},
             checked(A, HIZALI, kh, c1_height=2.5), c1_height=2.5),
        done("Gizli altlık da denetlenir", [entry(A, "denetim-hizali.ifc", shown=False)], {"id": A},
             checked(A, HIZALI, kh)),
        done("Listenin ikinci altlığı", [hz, entry(B, "denetim-kaymis.ifc")], {"id": B}, checked(B, KAYMIS, kk)),
        done("Başka sistemdeki model başlangıç noktasında denetlenir", [entry(A, "denetim-sistem.ifc", "origin")],
             {"id": A}, checked(A, HIZALI, ids["denetim-sistem.ifc"])),
        done("Yükseklik sistemi söylenmeyen projede kotlar aynı datumda sayılır, uyarı bunu söyler", [hz], {"id": A},
             checked(A, HIZALI, kh), heights=False,
             warnings=[{"code": "underlay_datum_unsaid", "message": "$any", "path": "id"}]),
        done("Başlangıç noktasındaki modelde datum sorulmaz", [entry(A, "denetim-datum.ifc", "origin")], {"id": A},
             checked(A, HIZALI, ids["denetim-datum.ifc"]), heights=False),
        refused("Farklı yükseklik datumu geometriden önce durdurur", [entry(A, "denetim-datum.ifc")], {"id": A},
                "model_underlay_not_placed", "id"),
        refused("Başka koordinat sistemi geometriden önce durdurur", [entry(A, "denetim-sistem.ifc")], {"id": A},
                "model_underlay_not_placed", "id"),
        refused("Sistemsiz projede harita dönüşümüyle yerleştirilmez", [hz], {"id": A}, "model_underlay_not_placed",
                "id", srid=0),
        refused("Eklendiğinden beri değişen dosya", [entry(A, "denetim-hizali.ifc", sha="0" * 64)], {"id": A},
                "model_underlay_changed", "id"),
        refused("Olmayan dosya", [entry(A, "denetim-hizali.ifc", sha="0" * 64) | {"file": "$fixture:" + REL + "yok.ifc"}],
                {"id": A}, "file_not_found", "id"),
        refused("IFC olmayan dosya", [entry(A, "var.ifc", file="arch/v1/ifc/var.ifc")], {"id": A},
                "model_underlay_unreadable", "id"),
        refused("Kimlik UUID değil", [hz], {"id": "altlik-1"}, "invalid_uid", "id"),
        refused("Büyük harfli kimlik", [hz], {"id": A.upper()}, "invalid_uid", "id"),
        refused("Olmayan altlık", [hz], {"id": B}, "model_underlay_not_found", "id"),
        refused("Altlıksız çizim", [], {"id": A}, "model_underlay_not_found", "id"),
        refused("Negatif tolerans", [hz], {"id": A, "tolerance": -0.001}, "invalid_tolerance", "tolerance"),
        refused("0,1 mm'den küçük tolerans", [hz], {"id": A, "tolerance": 0.00005}, "invalid_tolerance", "tolerance"),
        refused("0,5 m'den büyük tolerans", [hz], {"id": A, "tolerance": 0.6}, "invalid_tolerance", "tolerance"),
        refused("Bilinmeyen kimlik toleranstan önce", [hz], {"id": B, "tolerance": 0.6}, "model_underlay_not_found",
                "id"),
        refused("Aralık dışı tolerans dosyadan önce", [entry(A, "denetim-hizali.ifc", sha="0" * 64)],
                {"id": A, "tolerance": 0.6}, "invalid_tolerance", "tolerance"),
    ]
    doc = {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
           "title": "Altlık modeli denetle: katlar, akslar, eşleşen ve çakışan elemanlar; başka sistem ya da datum "
                    "geometriden önce durdurur",
           "note": "ADR 8102. Dosyaları IfcOpenShell yazar; beklenenler scripts/fixtures/underlay_check_cases.py'nin "
                   "ADR'den yazılmış kurallarından, KentOS kodu yok.",
           "setup": setup([hz]), "cases": cases}
    out = {OUT: json.dumps(doc, ensure_ascii=False, indent=2) + "\n"}
    for name, text in texts.items():
        out[FILES / name] = text
    stale = []
    for path, text in out.items():
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                stale.append(path.name)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
    if stale:
        print("güncel değil: " + ", ".join(stale), file=sys.stderr)
        return 1
    print(("güncel: " if check else "yazıldı: ") + f"{len(cases)} durum, " + ", ".join(p.name for p in out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
