#!/usr/bin/env python3
"""The shared cases of cad.sources.reconcile (docs/adr/0538, 0553, plan/gis.md CBS-37): a bound layer and its table,
both changed since they last matched, put together object by object; fields merged against the base, related changes
resolved as groups, several layers of one file in one call.

    python3 scripts/fixtures/reconcile_cases.py           # writes fixtures/commands/v1/cad.sources.reconcile.json
    python3 scripts/fixtures/reconcile_cases.py --check   # compares
    python3 scripts/fixtures/reconcile_cases.py --files   # GDAL writes the four files below
    python3 scripts/fixtures/reconcile_cases.py --written # OGR reads kentos-uzlasti.gpkg (KENTOS_WRITE_GPKG=1 cargo
                                                          # test -p kentos-native-application --test all reconcile)

No KentOS code. The files are written here with GDAL's OGR. `uzlas.gpkg` holds ten parcels; `uzlas-qgis.gpkg` is the
same file edited as a QGIS user would (an attribute, a geometry moved, rows deleted, a row added). `uzlas-komsu.gpkg`
holds four adjacent parcels (`komsu`, side by side, sharing their edges) and three buildings (`bina`, each pointing at
its parcel by the parcel's lasting id); `uzlas-komsu-qgis.gpkg` is that file as QGIS edits it. A case copies the first
file of a pair over a temporary file, binds it, edits the layer through the commands, copies the second over the file
(the edit in QGIS) and reconciles. A table's rows are read with gpkg_import_cases.py's OGR reader, as the import reads
them.

The rule (ADR 0538 §2, 0553) is written here from the ADRs: against the objects as they were when the layer and the table
last matched (the first file, as the link read it), each side either did nothing, added, changed or removed an object.
One side only: that side wins. Both removed, or both changed to the same kind, geometry and attributes: agreed. One
removed what the other changed: a conflict (its attributes are all the holder's). Both changed it: three-way merge by
field: an attribute changed by one side only takes that side's value, one changed by both to different values is a
conflict (both values); the kind and geometry are one more field of the same rule; no conflict left, the object is the
merge (neither side's own, or one's when it holds the other's changes too).
Related changes (ADR 0553 §2) are grouped: two objects that shared a vertex at the base which both sides moved
(neither side kept it in both objects), a parent one side removed and a child (a relation's) the other added or changed
referring to it; a group is taken whole, to one side, by any member's `take` or by `rest`. Conflicts are listed in the
lasting ids' byte order. The counts into the table are those of the write that follows; an unresolved conflict is left
as the table holds it.
"""
import copy
import hashlib
import json
import shutil
import sys
import uuid
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import bound_sources_cases as bs  # noqa: E402
import gpkg_import_cases as gi  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
OUT = ROOT / "fixtures/commands/v1/cad.sources.reconcile.json"
A = "uzlas.gpkg"
B = "uzlas-qgis.gpkg"
A2 = "uzlas-komsu.gpkg"
B2 = "uzlas-komsu-qgis.gpkg"
WRITTEN = "kentos-uzlasti.gpkg"
WRITTEN_K = "kentos-uzlasti-komsu.gpkg"
LAYER = "import-parsel"
KOMSU = "import-komsu"
BINA = "import-bina"
ANY = "$any"
MALIK = ["Ahmet", "Ayşe", "Can", "Deniz", "Ece", "Fatma", "Gül", "Hakan", "İpek", "Kemal"]
X0, Y0 = 487000.0, 4420000.0
RELATION = {"id": "bina-komsu", "name": "Parselin binaları", "parent": KOMSU, "child": BINA, "field": "komsu"}


def square(x, y, side=20.0):
    return [(x, y), (x + side, y), (x + side, y + side), (x, y + side), (x, y)]


def komsu_uid(fid):
    return str(uuid.uuid5(gi.NAMESPACE, f"gpkg:komsu:{fid}"))


def write_files():
    """A: ten parcels in TUREF / TM36; B: A as a QGIS user edited it. A2: four adjacent parcels and three buildings;
    B2: A2 as QGIS edited it."""
    from osgeo import ogr, osr
    a = DIR / A
    if a.exists():
        a.unlink()
    drv = ogr.GetDriverByName("GPKG")
    ds = drv.CreateDataSource(str(a))
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5256)
    lyr = ds.CreateLayer("parsel", srs, ogr.wkbMultiPolygon, ["FID=fid", "GEOMETRY_NAME=geom"])
    f_ada = ogr.FieldDefn("ada", ogr.OFTString)
    f_ada.SetWidth(20)
    lyr.CreateField(f_ada)
    lyr.CreateField(ogr.FieldDefn("parsel_no", ogr.OFTInteger))
    f_malik = ogr.FieldDefn("malik", ogr.OFTString)
    f_malik.SetWidth(40)
    lyr.CreateField(f_malik)

    def poly(ring):
        r = ogr.Geometry(ogr.wkbLinearRing)
        for px, py in ring:
            r.AddPoint_2D(px, py)
        p = ogr.Geometry(ogr.wkbPolygon)
        p.AddGeometry(r)
        m = ogr.Geometry(ogr.wkbMultiPolygon)
        m.AddGeometry(p)
        return m

    for i in range(1, 11):
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(i)
        f.SetField("ada", str(100 + i))
        f.SetField("parsel_no", i)
        f.SetField("malik", MALIK[i - 1])
        f.SetGeometry(poly(square(487000.0 + 30 * (i - 1), 4420000.0)))
        lyr.CreateFeature(f)
    ds = None
    b = DIR / B
    shutil.copyfile(a, b)
    ds = ogr.Open(str(b), 1)
    lyr = ds.GetLayerByName("parsel")

    def edit(fid, **values):
        f = lyr.GetFeature(fid)
        for k, v in values.items():
            if k == "geom":
                f.SetGeometry(v)
            else:
                f.SetField(k, v)
        lyr.SetFeature(f)

    edit(1, ada="201")
    edit(2, geom=poly(square(487035.0, 4420000.0)))
    lyr.DeleteFeature(3)
    edit(4, malik="Zehra")
    edit(5, ada="205")
    lyr.DeleteFeature(6)
    edit(7, ada="207")
    edit(10, malik="Ortak")
    f = ogr.Feature(lyr.GetLayerDefn())
    f.SetField("ada", "111")
    f.SetField("parsel_no", 11)
    f.SetField("malik", "Yeni")
    f.SetGeometry(poly(square(487300.0, 4420000.0)))
    lyr.CreateFeature(f)
    ds = None

    # A2: four parcels side by side (they share their edges) and three buildings, one on each of the first three.
    a2 = DIR / A2
    if a2.exists():
        a2.unlink()
    ds = drv.CreateDataSource(str(a2))
    k = ds.CreateLayer("komsu", srs, ogr.wkbMultiPolygon, ["FID=fid", "GEOMETRY_NAME=geom"])
    for name in ("ad", "tur", "not"):
        d = ogr.FieldDefn(name, ogr.OFTString)
        d.SetWidth(30)
        k.CreateField(d)
    for i in range(1, 5):
        f = ogr.Feature(k.GetLayerDefn())
        f.SetFID(i)
        f.SetField("ad", f"K{i}")
        f.SetField("tur", "arsa")
        f.SetField("not", "-")
        f.SetGeometry(poly(square(X0 + 20 * (i - 1), Y0)))
        k.CreateFeature(f)
    n = ds.CreateLayer("bina", srs, ogr.wkbMultiPolygon, ["FID=fid", "GEOMETRY_NAME=geom"])
    for name in ("komsu", "kat", "not"):
        d = ogr.FieldDefn(name, ogr.OFTString)
        d.SetWidth(40)
        n.CreateField(d)
    for i in range(1, 4):
        f = ogr.Feature(n.GetLayerDefn())
        f.SetFID(i)
        f.SetField("komsu", komsu_uid(i))
        f.SetField("kat", "1")
        f.SetField("not", "-")
        f.SetGeometry(poly(square(X0 + 20 * (i - 1) + 5, Y0 + 5, 10.0)))
        n.CreateFeature(f)
    ds = None
    b2 = DIR / B2
    shutil.copyfile(a2, b2)
    ds = ogr.Open(str(b2), 1)
    k = ds.GetLayerByName("komsu")
    n = ds.GetLayerByName("bina")
    # QGIS: the top right corner of the second parcel moved up (the third, which shares it, left alone), the first
    # parcel's note changed, a building added on the fourth parcel and the third building's floors and note changed.
    f = k.GetFeature(2)
    f.SetGeometry(poly([(X0 + 20, Y0), (X0 + 40, Y0), (X0 + 40, Y0 + 25), (X0 + 20, Y0 + 20), (X0 + 20, Y0)]))
    k.SetFeature(f)
    f = k.GetFeature(1)
    f.SetField("not", "qgis")
    k.SetFeature(f)
    f = n.GetFeature(3)
    f.SetField("kat", "9")
    f.SetField("not", "q")
    n.SetFeature(f)
    f = ogr.Feature(n.GetLayerDefn())
    f.SetField("komsu", komsu_uid(4))
    f.SetField("kat", "3")
    f.SetField("not", "-")
    f.SetGeometry(poly(square(X0 + 60 + 5, Y0 + 5, 10.0)))
    n.CreateFeature(f)
    ds = None


def rows(name, table="parsel"):
    t = next(t for t in gi.read(name)["tables"] if t["name"] == table)
    return {r["fid"]: r for r in t["rows"]}


def obj(row):
    """An object as the layer holds a row: kind, geometry, attributes (gpkg_import_cases.py's reading)."""
    return {"kind": row["kind"], "attrs": dict(row["attrs"]), **copy.deepcopy(row["geometry"])}


def essence(o):
    return {k: v for k, v in o.items() if k not in ("layerId",)}


def same_object(a, b):
    """Kind, geometry and attributes the same (numbers as numbers)."""
    return json.dumps(essence(a), sort_keys=True) == json.dumps(essence(b), sort_keys=True)


def shape(o):
    return json.dumps({k: v for k, v in o.items() if k not in ("layerId", "attrs")}, sort_keys=True)


def vertices(o):
    """The exact points of an object: every {x, y} under the geometry's keys."""
    out = set()

    def walk(v):
        if isinstance(v, dict):
            if "x" in v and "y" in v and len(v) <= 4:
                out.add((v["x"], v["y"]))
                return
            for x in v.values():
                walk(x)
        elif isinstance(v, list):
            for x in v:
                walk(x)

    for key in ("p", "pts", "parts", "holes", "a", "b"):
        if key in o:
            walk(o[key])
    return out


# ── The steps ────────────────────────────────────────────────────────────────

def copy_step(name, tmp):
    return {"op": "copyFile", "from": f"$fixture:geopackage/v1/{name}", "to": f"$tmp:{tmp}"}


def link_step(tmp, table="parsel"):
    return bs.other("cad.sources.edit", {"edit": {"kind": "link", "path": f"$tmp:{tmp}", "table": table}})


class Counter:
    """The next free slot of the document, shared by its layers."""

    def __init__(self, n):
        self.n = n

    def take(self):
        n = self.n
        self.n += 1
        return n


class Layer:
    """A bound layer as the steps leave it: objects by slot, each with its uid (None: a new one)."""

    def __init__(self, table="parsel", file=A, layer_id=LAYER, start=2, counter=None):
        a = rows(file, table)
        self.table, self.file, self.id, self.start = table, file, layer_id, start
        self.slots = {}
        self.uid = {}
        for k, fid in enumerate(sorted(a)):
            slot = start + k
            self.slots[slot] = obj(a[fid])
            self.uid[slot] = a[fid]["uid"]
        self.counter = counter or Counter(start + len(a))

    @property
    def next(self):
        return self.counter.n

    def slot_of(self, fid):
        u = rows(self.file, self.table)[fid]["uid"]
        return next(s for s, x in self.uid.items() if x == u)

    def set(self, fid, attrs):
        s = self.slot_of(fid)
        for k, v in attrs.items():
            if v is None:
                self.slots[s]["attrs"].pop(k, None)
            else:
                self.slots[s]["attrs"][k] = v
        return bs.other("cad.entities.set", {"uids": [self.uid[s]], "attrs": attrs, "operation": "attributes"})

    def delete(self, fid):
        s = self.slot_of(fid)
        u = self.uid.pop(s)
        del self.slots[s]
        return bs.other("cad.entities.delete", {"uids": [u]})

    def move(self, fid, pts):
        """The polygon's vertices replaced (a grip edit)."""
        s = self.slot_of(fid)
        pts = [{"x": x, "y": y} for x, y in pts]
        self.slots[s]["pts"] = pts
        return bs.other("cad.entities.edit", {"operation": "grip", "changes": [
            {"kind": "update", "uid": self.uid[s], "geometry": {"kind": "polygon", "pts": pts}}]})

    def add_polygon(self, x, y):
        pts = [{"x": px, "y": py} for px, py in square(x, y)[:-1]]
        s = self.counter.take()
        self.slots[s] = {"kind": "polygon", "attrs": {}, "pts": pts}
        self.uid[s] = None
        return bs.other("cad.polygon.create", {"layerId": self.id, "pts": pts})


EDITS = [("set", 4, {"malik": "Veli"}), ("set", 5, {"parsel_no": "50"}), ("set", 6, {"malik": "Mehmet"}),
         ("delete", 7, None), ("set", 8, {"malik": "Selin"}), ("delete", 9, None), ("set", 10, {"malik": "Ortak"}),
         ("add", (487400.0, 4420000.0), None)]


def edit_steps(layer, edits=EDITS):
    out = []
    for what, fid, attrs in edits:
        if what == "set":
            out.append(layer.set(fid, attrs))
        elif what == "delete":
            out.append(layer.delete(fid))
        elif what == "move":
            out.append(layer.move(fid, attrs))
        else:
            out.append(layer.add_polygon(*fid))
    return out


# ── The rule ────────────────────────────────────────────────────────────────

def did(b, now):
    if b is None:
        return "added" if now is not None else None
    if now is None:
        return "removed"
    return None if same_object(b, now) else "changed"


def field(n, lv, tv):
    f = {"name": n}
    if lv is not None:
        f["layer"] = lv
    if tv is not None:
        f["table"] = tv
    return f


def three_way(b, l, t, prefer=None):
    """The merge of l and t against b: (object, from_layer, from_table, geometry side, conflicting fields, geometry
    conflict). A conflicting attribute and a conflicting geometry go the way `prefer` says (the layer's by default)."""
    ba, la, ta = b["attrs"], l["attrs"], t["attrs"]
    attrs, from_layer, from_table, conflicts = {}, [], [], []
    for n in sorted(set(la) | set(ta)):
        bv, lv, tv = ba.get(n), la.get(n), ta.get(n)
        lc, tc = lv != bv, tv != bv
        if not lc and not tc:
            pick = lv
        elif lc and not tc:
            pick, _ = lv, from_layer.append(n)
        elif tc and not lc:
            pick, _ = tv, from_table.append(n)
        elif lv == tv:
            pick = lv
        else:
            conflicts.append(field(n, lv, tv))
            pick = tv if prefer == "table" else lv
        if pick is not None:
            attrs[n] = pick
    lm, tm = shape(l) != shape(b), shape(t) != shape(b)
    geometry, clash = None, False
    if lm and not tm:
        src, geometry = l, "layer"
    elif tm and not lm:
        src, geometry = t, "table"
    elif lm and tm and shape(l) != shape(t):
        src, clash = (t if prefer == "table" else l), True
    else:
        src = l
    merged = {**{k: v for k, v in src.items() if k not in ("attrs", "layerId")}, "attrs": attrs}
    return merged, from_layer, from_table, geometry, conflicts, clash


def fields_between(l, t):
    la, ta = l["attrs"], t["attrs"]
    return [field(n, la.get(n), ta.get(n)) for n in sorted(set(la) | set(ta)) if la.get(n) != ta.get(n)]


def fields_of_one(o, side):
    return [{"name": n, side: v} for n, v in sorted(o["attrs"].items())]


def classify(o):
    b, l, t = o["b"], o["l"], o["t"]
    ld, td = o["ld"], o["td"]
    o["way"] = "skip"
    if ld is None and td is None:
        return
    if td is None:
        o["way"] = "layer"
    elif ld is None:
        o["way"] = "table"
    elif ld == "removed" and td == "removed":
        o["way"] = "agreed"
    elif "removed" in (ld, td):
        held, side = (t, "table") if ld == "removed" else (l, "layer")
        o["way"] = "conflict"
        o["own"] = {"kind": "removedInLayer" if ld == "removed" else "removedInTable", "geometry": True,
                    "fields": fields_of_one(held, side)}
    elif same_object(l, t):
        o["way"] = "agreed"
    elif b is None:
        o["way"] = "conflict"
        o["own"] = {"kind": "changed", "geometry": shape(l) != shape(t), "fields": fields_between(l, t)}
    else:
        merged, from_layer, from_table, geometry, conflicts, clash = three_way(b, l, t)
        if conflicts or clash:
            o["way"] = "conflict"
            o["own"] = {"kind": "changed", "geometry": clash, "fields": conflicts}
        elif same_object(merged, l):
            o["way"] = "layer"
        elif same_object(merged, t):
            o["way"] = "table"
        else:
            o["way"] = "merged"
            o["merge"] = (merged, from_layer, from_table, geometry)


def groups_of(objs, relations):
    """Groups of related changes: lists of indexes into objs (in the ids' order), with their reasons."""
    active = [i for i, o in enumerate(objs) if o["way"] not in ("skip", "agreed")]
    parent = list(range(len(objs)))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    reasons = {}
    ties = []
    held = {i: (vertices(objs[i]["b"]), vertices(objs[i]["l"]) if objs[i]["l"] else set(),
                vertices(objs[i]["t"]) if objs[i]["t"] else set()) for i in active if objs[i]["b"] is not None}
    for k, i in enumerate(sorted(held)):
        for j in sorted(held)[k + 1:]:
            bi, li, ti = held[i]
            bj, lj, tj = held[j]
            for q in bi & bj:
                if not (q in li and q in lj) and not (q in ti and q in tj):
                    ties.append((i, j, "sharedVertex"))
                    break
    by_uid = {objs[i]["uid"]: i for i in active}
    for r in relations:
        for ci in active:
            c = objs[ci]
            if c["layer_id"] != r["child"]:
                continue
            refs = lambda e: {v for v in (e["attrs"].get(r["field"], "").split(";") if e else []) if v.strip()}  # noqa: E731
            refs_l = {x.strip() for x in refs(c["l"])}
            refs_t = {x.strip() for x in refs(c["t"])}
            for text in sorted(refs_l | refs_t):
                pi = by_uid.get(text)
                if pi is None or objs[pi]["layer_id"] != r["parent"]:
                    continue
                p = objs[pi]
                new = lambda d: d in ("added", "changed")  # noqa: E731
                if (p["ld"] == "removed" and p["td"] != "removed" and new(c["td"]) and text in refs_t) or (
                        p["td"] == "removed" and p["ld"] != "removed" and new(c["ld"]) and text in refs_l):
                    ties.append((ci, pi, "relation"))
    for i, j, why in ties:
        parent[max(find(i), find(j))] = min(find(i), find(j))
        reasons.setdefault(i, set()).add(why)
        reasons.setdefault(j, set()).add(why)
    comps = {}
    for i, j, _ in ties:
        for x in (i, j):
            comps.setdefault(find(x), set()).add(x)
    out = []
    for root in sorted(comps):
        members = sorted(comps[root])
        why = set().union(*(reasons[m] for m in members))
        out.append((members, [w for w in ("sharedVertex", "relation") if w in why]))
    return out


def solve(layers, tables, bases, relations=(), take=None, rest=None):
    """The answer of a reconcile of `layers` against the files `tables` (per layer: uid → object as the table holds
    it now) with `bases` (uid → object when they last matched), and what the layers and the tables become:
    (answer without source and revision, the objects by slot, their uids, the layer ids by slot)."""
    take = take or {}
    multi = len(layers) > 1
    objs = []
    for li, lay in enumerate(layers):
        base, table = bases[li], tables[li]
        mine = {lay.uid[s]: o for s, o in lay.slots.items() if lay.uid[s] is not None}
        for u in set(base) | set(mine) | set(table):
            b, l, t = base.get(u), mine.get(u), table.get(u)
            o = {"li": li, "layer_id": lay.id, "uid": u, "b": b, "l": l, "t": t, "ld": did(b, l), "td": did(b, t)}
            classify(o)
            objs.append(o)
    objs.sort(key=lambda o: uuid.UUID(o["uid"]).bytes)
    agreed = sum(1 for o in objs if o["way"] == "agreed")
    groups = groups_of(objs, relations)
    group_of = {m: g for g, (members, _) in enumerate(groups) for m in members}

    # The conflicts, in order, with the group each belongs to.
    conflicts, at = [], {}
    for i, o in enumerate(objs):
        if o["way"] != "conflict" and i not in group_of:
            continue
        c = {"object": o["uid"]}
        if multi:
            c["layer"] = o["layer_id"]
        if o["way"] == "conflict":
            c["kind"] = o["own"]["kind"]
            c["geometry"] = o["own"]["geometry"]
            fields = o["own"]["fields"]
        else:
            c["kind"] = "related"
            l, t = o["l"], o["t"]
            if l and t:
                c["geometry"], fields = shape(l) != shape(t), fields_between(l, t)
            else:
                c["geometry"], fields = True, fields_of_one(l or t, "layer" if l else "table")
        if i in group_of:
            c["group"] = objs[groups[group_of[i]][0][0]]["uid"]
        if fields:
            c["fields"] = fields
        at[i] = len(conflicts)
        conflicts.append(c)
    group_side = {}
    own_side = {}
    for u, side in take.items():
        i = next(i for i, o in enumerate(objs) if o["uid"] == u)
        if i in group_of:
            group_side[group_of[i]] = side
        else:
            own_side[i] = side

    def side_of(i):
        if i in group_of:
            return group_side.get(group_of[i], rest)
        return own_side.get(i, rest)

    for i, ci in at.items():
        s = side_of(i)
        if s:
            conflicts[ci]["side"] = s
    unresolved = sum(1 for c in conflicts if "side" not in c)

    # The layers after: the table's side taken into them.
    after = {}
    uid_after = {}
    layer_of = {}
    for lay in layers:
        for s, o in lay.slots.items():
            after[s] = copy.deepcopy(o)
            uid_after[s] = lay.uid[s]
            layer_of[s] = lay.id
    frm = [{"added": 0, "modified": 0, "removed": 0} for _ in layers]
    nxt = max((lay.next for lay in layers), default=0)
    for i, o in enumerate(objs):
        way = o["way"]
        if way in ("skip", "agreed"):
            continue
        s_ = side_of(i)
        counted = not (i in at and s_ is None)
        if i in group_of:
            target = "layer" if s_ == "layer" else "table"
        elif way == "layer":
            target = "layer"
        elif way == "table":
            target = "table"
        elif way == "merged":
            target = ("made", o["merge"][0])
        else:
            made = None
            if o["own"]["kind"] == "changed" and o["b"] is not None and s_ is not None:
                made = three_way(o["b"], o["l"], o["t"], s_)[0]
            target = ("made", made) if made is not None else ("layer" if s_ == "layer" else "table")
        slot = next((s for s, x in uid_after.items() if x == o["uid"]), None)
        li = o["li"]
        if target == "layer":
            continue
        if isinstance(target, tuple):
            m = target[1]
            if o["l"] is None or slot is None or same_object(o["l"], m):
                continue
            after[slot] = copy.deepcopy(m)
            if counted:
                frm[li]["modified"] += 1
            continue
        t = o["t"]
        if slot is not None and t is not None:
            if same_object(o["l"], t):
                continue
            after[slot] = copy.deepcopy(t)
            if counted:
                frm[li]["modified"] += 1
        elif t is not None:
            after[nxt] = copy.deepcopy(t)
            uid_after[nxt] = o["uid"]
            layer_of[nxt] = o["layer_id"]
            nxt += 1
            if counted:
                frm[li]["added"] += 1
        elif slot is not None:
            del after[slot]
            del uid_after[slot]
            del layer_of[slot]
            if counted:
                frm[li]["removed"] += 1

    # The write: the layers after against the tables.
    to = []
    for li, lay in enumerate(layers):
        c = {"added": 0, "modified": 0, "removed": 0}
        held = {uid_after[s]: o for s, o in after.items() if layer_of[s] == lay.id and uid_after[s] is not None}
        for u, o in held.items():
            if u not in tables[li]:
                c["added"] += 1
            elif not same_object(o, tables[li][u]):
                c["modified"] += 1
        c["added"] += sum(1 for s in after if layer_of[s] == lay.id and uid_after[s] is None)
        c["removed"] = sum(1 for u in tables[li] if u not in held)
        to.append(c)
    total = lambda cs: {k: sum(c[k] for c in cs) for k in ("added", "modified", "removed")}  # noqa: E731
    answer = {"fromTable": total(frm), "toTable": total(to), "agreed": agreed}
    merged = []
    for i, o in enumerate(objs):
        if o["way"] != "merged" or i in group_of:
            continue
        _, fl, ft, geometry = o["merge"]
        m = {"object": o["uid"]}
        if multi:
            m["layer"] = o["layer_id"]
        if fl:
            m["fromLayer"] = fl
        if ft:
            m["fromTable"] = ft
        if geometry:
            m["geometry"] = geometry
        merged.append(m)
    if merged:
        answer["merged"] = merged
    if groups:
        out = []
        for g, (members, why) in enumerate(groups):
            e = {"id": objs[members[0]]["uid"], "objects": [objs[m]["uid"] for m in members], "reasons": why}
            s = group_side.get(g, rest)
            if s:
                e["side"] = s
            out.append(e)
        answer["groups"] = out
    if multi:
        answer["layers"] = [{"layer": lay.id, "fromTable": frm[li], "toTable": to[li]}
                            for li, lay in enumerate(layers)]
    if conflicts:
        answer["conflicts"] = conflicts
    answer["unresolved"] = unresolved
    return answer, after, uid_after, layer_of


def reconcile(layer, table_name, take=None, rest=None):
    """The one-layer reconcile of the parcels: (answer, objects by slot, uids by slot)."""
    base = {r["uid"]: obj(r) for r in rows(A).values()}
    table = {r["uid"]: obj(r) for r in rows(table_name).values()}
    answer, after, uid_after, _ = solve([layer], [table], [base], take=take, rest=rest)
    return answer, after, uid_after


def answer_of(core, source):
    return {**core, "source": source, "revision": "$current"}


def fields_of(after, layer_of=None):
    return {str(s): {**o, "layerId": (layer_of or {}).get(s, LAYER)} for s, o in sorted(after.items())}


def bound(tmp, edits=EDITS, then=B):
    """The steps up to a reconcile: the first file bound, the layer edited, the file changed in QGIS."""
    layer = Layer()
    steps = [copy_step(A, tmp), link_step(tmp), *edit_steps(layer, edits)]
    if then:
        steps.append(copy_step(then, tmp))
    return layer, steps


def plan_case(name, tmp, inp=None, take_=None, rest=None, edits=EDITS, then=B):
    layer, steps = bound(tmp, edits, then)
    core, _, _ = reconcile(layer, then or A, take_, rest)
    inp = {"layer": LAYER, **(inp or {})}
    steps.append(bs.step(inp, bs.done(answer_of(core, bs.sha(then or A))), op="plan"))
    return bs.case(name, *steps), layer


def ids_after(after):
    return [1, *sorted(after)]


# ── The neighbours and buildings ────────────────────────────────────────────

def bound2(tmp, k_edits=(), b_edits=(), then=B2, relation=True):
    """The komsu and bina tables of the first file bound (slots 2–5, 6–8), the project's relation, each layer edited,
    the file changed in QGIS."""
    counter = Counter(2 + 4 + 3)
    k = Layer("komsu", A2, KOMSU, 2, counter)
    b = Layer("bina", A2, BINA, 6, counter)
    steps = [copy_step(A2, tmp), link_step(tmp, "komsu"), link_step(tmp, "bina")]
    if relation:
        steps.append(bs.other("cad.relations.edit", {"edit": {"kind": "create", "relation": copy.deepcopy(RELATION)}}))
    steps += edit_steps(k, k_edits)
    steps += edit_steps(b, b_edits)
    if then:
        steps.append(copy_step(then, tmp))
    return k, b, steps


def solve2(k, b, then=B2, take=None, rest=None, only_k=False, relation=True):
    layers = [k] if only_k else [k, b]
    names = [("komsu", A2, then or A2), ("bina", A2, then or A2)]
    bases = [{r["uid"]: obj(r) for r in rows(A2, t).values()} for t, _, _ in names[:len(layers)]]
    tables = [{r["uid"]: obj(r) for r in rows(n, t).values()} for t, _, n in names[:len(layers)]]
    return solve(layers, tables, bases, [RELATION] if relation else [], take, rest)


def uk(fid):
    return rows(A2, "komsu")[fid]["uid"]


def ub(fid):
    return rows(A2, "bina")[fid]["uid"]


# K2 and K3 share the corner (X0 + 40, Y0 + 20): the layer moves it in both; QGIS moved it in K2 only.
SHARED = (X0 + 40, Y0 + 20)
MOVED = (X0 + 42, Y0 + 22)
K2_MOVED = [(X0 + 20, Y0), (X0 + 40, Y0), MOVED, (X0 + 20, Y0 + 20)]
K3_MOVED = [(X0 + 40, Y0), (X0 + 60, Y0), (X0 + 60, Y0 + 20), MOVED]
K_EDITS = [("set", 1, {"ad": "Z1"}), ("move", 2, K2_MOVED), ("move", 3, K3_MOVED), ("delete", 4, None)]
B_EDITS = [("set", 2, {"kat": "5"}), ("set", 3, {"kat": "7"})]


def plan_case2(name, tmp, inp=None, take_=None, rest=None, k_edits=K_EDITS, b_edits=B_EDITS, then=B2,
               only_k=False, relation=True):
    k, b, steps = bound2(tmp, k_edits, b_edits, then, relation)
    core, _, _, _ = solve2(k, b, then, take_, rest, only_k, relation)
    inp = {"layer": KOMSU, **({} if only_k else {"with": [BINA]}), **(inp or {})}
    steps.append(bs.step(inp, bs.done(answer_of(core, bs.sha(then or A2))), op="plan"))
    return bs.case(name, *steps), (k, b)


def exec_case2(name, tmp, inp, take_=None, rest=None, k_edits=K_EDITS, b_edits=B_EDITS, then=B2, only_k=False,
               relation=True):
    k, b, steps = bound2(tmp, k_edits, b_edits, then, relation)
    core, after, _, layer_of = solve2(k, b, then, take_, rest, only_k, relation)
    inp = {"layer": KOMSU, **({} if only_k else {"with": [BINA]}), **inp}
    steps.append(bs.step(inp, bs.done(answer_of(core, ANY)),
                         {"ids": ids_after(after), "fields": fields_of(after, layer_of), "revision": "changed",
                          "canUndo": True}))
    zero = {"added": 0, "modified": 0, "removed": 0}
    again = {"fromTable": zero, "toTable": zero, "agreed": 0, "unresolved": 0, "source": ANY, "revision": "$current"}
    if not only_k:
        again["layers"] = [{"layer": KOMSU, "fromTable": zero, "toTable": zero},
                           {"layer": BINA, "fromTable": zero, "toTable": zero}]
    # The base is taken again: the two match now.
    steps.append(bs.step({k_: v for k_, v in inp.items() if k_ in ("layer", "with")}, bs.done(again), op="plan"))
    return bs.case(name, *steps)


def cases():
    a, b = rows(A), rows(B)
    u = {fid: r["uid"] for fid, r in a.items()}
    out = []
    c, _ = plan_case("plan iki yanın değişikliklerini, birleşenleri ve üç çatışmayı söyler, hiçbir şey yazmaz",
                     "uz-plan.gpkg")
    out.append(c)
    c, _ = plan_case("kalanlar katmanın yanına: tabloya yazılacaklar artar", "uz-katman.gpkg", {"rest": "layer"},
                     rest="layer")
    out.append(c)
    c, _ = plan_case("kalanlar tablonun yanına: katmana alınacaklar artar", "uz-tablo.gpkg", {"rest": "table"},
                     rest="table")
    out.append(c)
    take_ = {u[4]: "table", u[6]: "layer"}
    c, _ = plan_case("çatışmalar tek tek, kalanlar katmanın yanına", "uz-tek.gpkg",
                     {"take": [{"object": u[4], "side": "table"}, {"object": u[6], "side": "layer"}],
                      "rest": "layer"}, take_=take_, rest="layer")
    out.append(c)
    c, _ = plan_case("yalnız tablo değişti: hepsi katmana alınır, çatışma yok", "uz-yalniz-tablo.gpkg", edits=[])
    out.append(c)
    c, _ = plan_case("yalnız katman değişti: hepsi tabloya yazılır", "uz-yalniz-katman.gpkg", then=None)
    out.append(c)
    c, _ = plan_case("hiçbir yan değişmedi: hepsi sıfır", "uz-degismedi.gpkg", edits=[], then=None)
    out.append(c)
    c, _ = plan_case("ikisi de sildiyse uyuşur", "uz-ikisi-sildi.gpkg", edits=[("delete", 3, None),
                                                                              ("delete", 6, None)])
    out.append(c)

    # Fields merged against the base (ADR 0553 §1).
    c, _ = plan_case("farklı alanlar birleşir: katmanın ve tablonun öznitelikleri ikisi de kalır",
                     "uz-alan-birlesir.gpkg", {"rest": "table"}, rest="table",
                     edits=[("set", 1, {"malik": "Yeni1"}), ("set", 5, {"parsel_no": "50"})])
    out.append(c)
    c, _ = plan_case("öznitelik ve geometri ayrı yanlardan birleşir", "uz-geometri-oznitelik.gpkg",
                     {"rest": "table"}, rest="table", edits=[("set", 2, {"malik": "Yeni2"})])
    out.append(c)
    c, _ = plan_case("aynı alan iki yanda farklı değişirse yalnız o alan çatışır, öbürleri birleşir",
                     "uz-alan-catisir.gpkg", edits=[("set", 1, {"ada": "999", "malik": "Q"})])
    out.append(c)
    for side in ("layer", "table"):
        c, _ = plan_case(f"çatışan alan seçilen yandan, öbür alanlar birleşmiş gelir ({side})",
                         f"uz-alan-cozum-{side}.gpkg", {"take": [{"object": u[1], "side": side}], "rest": side},
                         take_={u[1]: side}, rest=side, edits=[("set", 1, {"ada": "999", "malik": "Q"})])
        out.append(c)
    c, _ = plan_case("aynı alana iki yan da aynı değeri yazdıysa uyuşur, çatışma değil", "uz-ayni-deger.gpkg",
                     edits=[("set", 1, {"ada": "201"})])
    out.append(c)
    c, _ = plan_case("bir yan alanı kaldırdı, öbürü başka alanı değiştirdi: birleşir", "uz-alan-kaldir.gpkg",
                     {"rest": "layer"}, rest="layer",
                     edits=[("set", 5, {"malik": None}), ("set", 1, {"parsel_no": "11"})])
    out.append(c)

    # Executing with every conflict resolved: the table's side into the layer, the layer's to the file.
    for name, tmp, rest in (("yürütme: kalanlar tablonun yanına, katman tablonun hâlini alır", "uz-yurut-tablo.gpkg",
                             "table"),
                            ("yürütme: kalanlar katmanın yanına, dosyaya katmanınkiler yazılır", "uz-yurut-katman.gpkg",
                             "layer")):
        layer, steps = bound(tmp)
        core, after, _ = reconcile(layer, B, rest=rest)
        steps.append(bs.step({"layer": LAYER, "rest": rest}, bs.done(answer_of(core, ANY)),
                             {"ids": ids_after(after), "fields": fields_of(after), "revision": "changed",
                              "canUndo": True}))
        # The base is taken again: the two match now.
        steps.append(bs.step({"layer": LAYER},
                             bs.done({"fromTable": {"added": 0, "modified": 0, "removed": 0},
                                      "toTable": {"added": 0, "modified": 0, "removed": 0}, "agreed": 0,
                                      "unresolved": 0, "source": ANY, "revision": "$current"}), op="plan"))
        out.append(bs.case(name, *steps))
    layer, steps = bound("uz-birlesme-yurut.gpkg", edits=[("set", 1, {"malik": "Yeni1"}), ("set", 2, {"malik": "Yeni2"})])
    core, after, _ = reconcile(layer, B, rest="layer")
    steps.append(bs.step({"layer": LAYER, "rest": "layer"}, bs.done(answer_of(core, ANY)),
                         {"ids": ids_after(after), "fields": fields_of(after), "revision": "changed",
                          "canUndo": True}))
    out.append(bs.case("yürütme: birleşen nesne iki yanın değişikliğini katmanda ve dosyada taşır", *steps))
    layer, steps = bound("uz-geri.gpkg")
    core, after, _ = reconcile(layer, B, rest="table")
    before = fields_of(layer.slots)
    steps.append(bs.step({"layer": LAYER, "rest": "table"}, bs.done(answer_of(core, ANY)),
                         {"revision": "changed", "canUndo": True}))
    steps.append({"op": "undo", "returns": "Uzlaştır: parsel",
                  "expect": {"ids": ids_after(layer.slots), "fields": before, "revision": "changed"}})
    out.append(bs.case("geri almak katmanı uzlaştırmadan önceki hâline döndürür", *steps))
    layer, steps = bound("uz-yalniz-tablo-yurut.gpkg", edits=[])
    core, after, _ = reconcile(layer, B)
    steps.append(bs.step({"layer": LAYER}, bs.done(answer_of(core, ANY)),
                         {"ids": ids_after(after), "fields": fields_of(after), "revision": "changed",
                          "canUndo": True}))
    out.append(bs.case("çatışma yoksa yan seçmeden yürür: katman tablonun hâlini alır", *steps))
    c, _ = plan_case("geçerleme: yan seçilmişse yürür", "uz-gecerle.gpkg", {"rest": "layer"}, rest="layer")
    c["steps"][-1] = bs.step({"layer": LAYER, "rest": "layer"}, bs.done(None), op="validate")
    out.append(c)

    # Related changes are one group (ADR 0553 §2): neighbours whose shared corner both moved, a parent one side
    # removed and a child the other added.
    c, _ = plan_case2("birlikte değişenler gruptur: ortak köşeyi iki yan da taşıdı, silinen üst kayıtla eklenen alt kayıt",
                      "uz-grup-plan.gpkg")
    out.append(c)
    c, _ = plan_case2("grubun nesneleri tek yanla çözülür: kalanlar katmanın yanına", "uz-grup-katman.gpkg",
                      {"rest": "layer"}, rest="layer")
    out.append(c)
    c, _ = plan_case2("grubun nesneleri tek yanla çözülür: kalanlar tablonun yanına", "uz-grup-tablo.gpkg",
                      {"rest": "table"}, rest="table")
    out.append(c)
    take_ = {uk(3): "table"}
    c, _ = plan_case2("bir üyeye seçilen yan bütün gruba geçer, kalanlar katmanın yanına", "uz-grup-uye.gpkg",
                      {"take": [{"object": uk(3), "side": "table"}], "rest": "layer"}, take_=take_, rest="layer")
    out.append(c)
    take_ = {uk(2): "layer", uk(4): "table", ub(3): "table"}
    c, _ = plan_case2("grup ve çatışmalar ayrı ayrı çözülür", "uz-grup-ayri.gpkg",
                      {"take": [{"object": uk(2), "side": "layer"}, {"object": uk(4), "side": "table"},
                                {"object": ub(3), "side": "table"}]}, take_=take_)
    out.append(c)
    out.append(exec_case2("yürütme: gruplar kalanların yanına, tek işlemde iki katman", "uz-grup-yurut-katman.gpkg",
                          {"rest": "layer"}, rest="layer"))
    out.append(exec_case2("yürütme: gruplar tablonun yanına, silinen üst kayıt geri gelir", "uz-grup-yurut-tablo.gpkg",
                          {"rest": "table"}, rest="table"))
    c, _ = plan_case2("yalnız katman değişti: ortak köşeyi yalnız o taşıdı, grup yok", "uz-grup-yalniz-katman.gpkg",
                      then=None, k_edits=[("move", 2, K2_MOVED), ("move", 3, K3_MOVED)], b_edits=[])
    out.append(c)
    c, _ = plan_case2("ortak köşeye dokunmayan taşımalar grup değildir", "uz-grup-uzak.gpkg",
                      k_edits=[("move", 3, [(X0 + 40, Y0), (X0 + 60, Y0), (X0 + 62, Y0 + 20), (X0 + 40, Y0 + 20)])],
                      b_edits=[])
    out.append(c)
    c, _ = plan_case2("silinen üst kayıtta alt kayıt yoksa grup yok: yalnız tablonun eklediği alt kayıt gelir",
                      "uz-grup-iliskisiz.gpkg", k_edits=[("delete", 4, None)], b_edits=[], relation=False)
    out.append(c)
    c, _ = plan_case2("tek katman da aynı çözümü kullanır, katman adı cevapta yoktur", "uz-tek-katman.gpkg",
                      {"rest": "layer"}, rest="layer", only_k=True, k_edits=[("set", 1, {"ad": "Z1"})], b_edits=[],
                      relation=False)
    out.append(c)

    # Refusals.
    unresolved_msg = ("3 çatışma çözülmedi; hiçbir şey yazılmadı. Her biri için take ile ya da kalanlar için rest ile "
                      "bir yan seçin (layer: katmanınki, table: tablonunki).")
    layer, steps = bound("uz-cozulmedi.gpkg")
    steps.append(bs.step({"layer": LAYER}, bs.conflict("unresolved_conflicts", unresolved_msg, "take")))
    out.append(bs.case("çözülmemiş çatışmayla yürümez, hiçbir şey yazılmaz", *steps))
    layer, steps = bound("uz-cozulmedi-g.gpkg")
    steps.append(bs.step({"layer": LAYER}, bs.conflict("unresolved_conflicts", unresolved_msg, "take"),
                         op="validate"))
    out.append(bs.case("geçerleme çözülmemiş çatışmayı reddeder", *steps))
    layer, steps = bound("uz-cozulmedi-tek.gpkg")
    steps.append(bs.step({"layer": LAYER, "take": [{"object": u[4], "side": "layer"}]},
                         bs.conflict("unresolved_conflicts", unresolved_msg.replace("3 çatışma", "2 çatışma"), "take")))
    out.append(bs.case("biri çözülse de kalanlar yürütmeyi durdurur", *steps))
    k, bn, steps = bound2("uz-grup-cozulmedi.gpkg", K_EDITS, B_EDITS)
    steps.append(bs.step({"layer": KOMSU, "with": [BINA], "take": [{"object": uk(2), "side": "layer"}]},
                         bs.conflict("unresolved_conflicts",
                                     "3 çatışma çözülmedi; hiçbir şey yazılmadı. Her biri için take ile ya da "
                                     "kalanlar için rest ile bir yan seçin (layer: katmanınki, table: tablonunki).",
                                     "take")))
    out.append(bs.case("bir grubun çözülmesi ötekileri çözmez", *steps))
    layer, steps = bound("uz-cakisma-degil.gpkg")
    steps.append(bs.step({"layer": LAYER, "take": [{"object": u[1], "side": "layer"}]},
                         bs.failed("not_a_conflict", f"{u[1]} bir çatışma değil; yan yalnız plan'ın conflicts'indeki "
                                   "nesneler için seçilir.", "take.0.object"), op="plan"))
    out.append(bs.case("yalnız tablonun değiştirdiği nesne için yan seçilmez", *steps))
    layer, steps = bound("uz-kimlik-degil.gpkg")
    steps.append(bs.step({"layer": LAYER, "take": [{"object": u[4], "side": "layer"},
                                                    {"object": "parsel-4", "side": "table"}]},
                         bs.failed("not_a_conflict", "parsel-4 bir çatışma değil; yan yalnız plan'ın conflicts'indeki "
                                   "nesneler için seçilir.", "take.1.object"), op="plan"))
    out.append(bs.case("kalıcı kimlik olmayan nesne", *steps))
    layer, steps = bound("uz-iki-kez.gpkg")
    steps.append(bs.step({"layer": LAYER, "take": [{"object": u[4], "side": "layer"},
                                                    {"object": u[4], "side": "table"}]},
                         bs.failed("invalid_input", f"{u[4]} için iki kez yan seçilmiş; her çatışma bir kez.",
                                   "take.1.object"), op="plan"))
    out.append(bs.case("bir çatışmaya iki kez yan seçilmez", *steps))
    k, bn, steps = bound2("uz-grup-bolunmez.gpkg", K_EDITS, B_EDITS)
    steps.append(bs.step({"layer": KOMSU, "with": [BINA], "take": [{"object": uk(2), "side": "layer"},
                                                                   {"object": uk(3), "side": "table"}]},
                         bs.failed("group_split", f"{uk(3)} nesnesi birlikte değişen bir grubun üyesi; grubun öbür "
                                   "nesneleri için başka yan seçilmiş. Grubun tamamı için bir yan seçin.",
                                   "take.1.object"), op="plan"))
    out.append(bs.case("bir grubun nesneleri farklı yanlara seçilemez", *steps))
    k, bn, steps = bound2("uz-with-iki-kez.gpkg", [], [], then=None)
    steps.append(bs.step({"layer": KOMSU, "with": [KOMSU]},
                         bs.failed("invalid_input", f"“{KOMSU}” katmanı birden çok kez verildi; her katman bir kez "
                                   "uzlaştırılır.", "with"), op="plan"))
    out.append(bs.case("aynı katman iki kez verilmez", *steps))
    k, bn, steps = bound2("uz-with-bagli-degil.gpkg", [], [], then=None)
    steps.append(bs.step({"layer": KOMSU, "with": ["cizim"]},
                         bs.failed("source_not_found", "“cizim” katmanı bir kaynağa bağlı değil; önce "
                                   "cad.sources.edit ile bağlayın.", "with"), op="plan"))
    out.append(bs.case("with'teki bağlı olmayan katman", *steps))
    layer = Layer()
    steps = [copy_step(A, "uz-with-dosya-a.gpkg"), link_step("uz-with-dosya-a.gpkg"),
             copy_step(A2, "uz-with-dosya-b.gpkg"), link_step("uz-with-dosya-b.gpkg", "komsu")]
    steps.append(bs.step({"layer": LAYER, "with": [KOMSU]},
                         bs.failed("transaction_split", f"“{KOMSU}” katmanı $tmp:uz-with-dosya-b.gpkg kaynağında, "
                                   f"“{LAYER}” katmanı $tmp:uz-with-dosya-a.gpkg dosyasında; birlikte uzlaştırılacak "
                                   "katmanlar tek GeoPackage dosyasında ya da tek veritabanında olmalı. Katmanları ayrı ayrı uzlaştırın.",
                                   "with"), op="plan"))
    out.append(bs.case("başka dosyadaki katman birlikte uzlaştırılmaz", *steps))
    layer, steps = bound("uz-kayip.gpkg", edits=[*EDITS, ("set", 8, {"durum": "x"})])
    core, _, _ = reconcile(layer, B, rest="layer")
    steps.append(bs.step({"layer": LAYER, "rest": "layer"},
                         bs.done(answer_of(core, bs.sha(B)),
                                 [{"code": "export_losses", "message": "parsel: katmanın alanı olmayan öznitelikler "
                                   "yazılmadı (durum); Alanlar penceresinden alan yapın.", "path": "layer"}]),
                         op="plan"))
    out.append(bs.case("alanı olmayan öznitelik yazılmaz, söylenir", *steps))
    layer, steps = bound("uz-nokta.gpkg", edits=[])
    steps.insert(2, bs.other("cad.point.create", {"layerId": LAYER, "p": gi.P(487500, 4420100)}))
    steps.insert(3, {"op": "captureUid", "id": 12, "as": "nokta"})
    steps.append(bs.step({"layer": LAYER}, bs.failed("geometry_mismatch", "$uid:nokta kimlikli nesne tabloya "
                                                     "yazılamaz: POINT geometrisi MULTIPOLYGON sütununa yazılamaz.",
                                                     "layer"), op="plan"))
    out.append(bs.case("tablonun sütununa uymayan nesne", *steps))
    out.append(bs.case("bağlı olmayan katman",
                       bs.step({"layer": "cizim"},
                               bs.failed("source_not_found", "“cizim” katmanı bir kaynağa bağlı değil; önce "
                                         "cad.sources.edit ile bağlayın.", "layer"), op="plan")))

    def link_to(layer_id, name=A, **over):
        s = {**bs.link("parsel", layer_id, name=name), "digest": "f" * 64, **over}
        return s

    bagli = bs.layer("bagli")
    out.append(bs.case("tabanı olmayan bağ uzlaştırılmaz",
                       bs.step({"layer": "bagli"},
                               bs.failed("no_source_base", "“bagli” katmanının dosyayla son eşleşmesinden tabanı yok "
                                         "(bu sürümden önce bağlanmış ya da 200000 nesneden büyük); iki yanın "
                                         "değişiklikleri ayrılamaz. Önce yazın (cad.sources.sync write) ya da "
                                         "tablodan okuyun (read).", "layer"), op="plan"),
                       setup_=bs.setup([bagli], sources=[link_to("bagli")])))
    out.append(bs.case("çizimde olmayan bağlı katman",
                       bs.step({"layer": "silinen"},
                               bs.failed("layer_not_found", "Bağlı “silinen” katmanı çizimde yok; bağı çözün "
                                         "(cad.sources.edit unlink).", "layer"), op="plan"),
                       setup_=bs.setup(sources=[link_to("silinen")])))
    out.append(bs.case("kilitli katman",
                       bs.step({"layer": "kilitli"},
                               bs.failed("layer_locked", "“kilitli” katmanı kilitli; kilidi Katmanlar panelinden "
                                         "açın.", "layer"), op="plan"),
                       setup_=bs.setup(sources=[link_to("kilitli")])))
    gone = link_to("bagli", path="$fixture:geopackage/v1/yok.gpkg")
    out.append(bs.case("yerinde olmayan dosya",
                       bs.step({"layer": "bagli"},
                               bs.failed("source_gone", f"{gone['path']} yerinde yok; dosyanın yeni yerini "
                                         "cad.sources.edit relink ile verin.", "layer"), op="plan"),
                       setup_=bs.setup([bagli], sources=[gone])))
    ro = link_to("bagli", writable=False)
    out.append(bs.case("salt okunur tablo",
                       bs.step({"layer": "bagli"},
                               bs.failed("source_readonly", f"{ro['path']} dosyasının “parsel” tablosuna yazılamaz "
                                         "(salt okunur); uzlaştırmak katmanın yanını yazar.", "layer"), op="plan"),
                       setup_=bs.setup([bagli], sources=[ro])))
    broken = {**link_to("bagli", name="bozuk.gpkg")}
    out.append(bs.case("okunmayan dosya",
                       bs.step({"layer": "bagli"},
                               {"status": "failed", "error": {"code": "import_unreadable", "message": ANY,
                                                               "path": "layer"}}, op="plan"),
                       setup_=bs.setup([bagli], sources=[broken])))
    missing = {**link_to("bagli"), "table": "yok"}
    out.append(bs.case("dosyada olmayan tablo",
                       bs.step({"layer": "bagli"},
                               {"status": "failed", "error": {"code": "table_not_found", "message": ANY,
                                                               "path": "layer"}}, op="plan"),
                       setup_=bs.setup([bagli], sources=[missing])))
    local = bs.setup([bagli], sources=[link_to("bagli")])
    local["settings"]["srid"] = 0
    out.append(bs.case("koordinat sistemi olmayan projede başka sistemdeki tablo",
                       bs.step({"layer": "bagli"},
                               bs.failed("crs_mismatch", "“bagli” eşitlenemez: tablo EPSG:5256 sisteminde; projenin "
                                         "koordinat sistemi yok (yerel proje), çevrilemez.", "layer"), op="plan"),
                       setup_=local))
    derived = {**link_to("bagli"), "table": "Parsel", "path": "$fixture:gis-derivation/v1/ada.gpkg",
               "revision": hashlib.sha256((ROOT / "fixtures/gis-derivation/v1/ada.gpkg").read_bytes()).hexdigest()}
    out.append(bs.case("çizimden türetilmiş tablo uzlaştırılmaz",
                       bs.step({"layer": "bagli"},
                               {"status": "failed", "error": {"code": "derived_dataset", "message": ANY,
                                                               "path": "layer"}}, op="plan"),
                       setup_=bs.setup([bagli], sources=[derived])))
    return out


def written():
    """kentos-uzlasti.gpkg (tests/all/sources.rs, rest: layer) by OGR: each row as the reconcile says."""
    path = DIR / WRITTEN
    if not path.exists():
        return f"{WRITTEN} yok: KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all reconcile"
    layer = Layer()
    edit_steps(layer)
    _, after, uid_after = reconcile(layer, B, rest="layer")
    got = rows(WRITTEN)
    by_uid = {r["uid"]: obj(r) for r in got.values()}
    want = {uid_after[s]: o for s, o in after.items() if uid_after[s] is not None}
    news = [o for s, o in after.items() if uid_after[s] is None]
    problems = []
    for u, o in want.items():
        g = by_uid.pop(u, None)
        if g is None:
            problems.append(f"{u}: satır yok")
        elif not same_object(o, g):
            problems.append(f"{u}: {g} ≠ {o}")
    for o in news:
        match = next((k for k, g in by_uid.items() if same_object(o, g)), None)
        if match is None:
            problems.append(f"yeni nesne yazılmamış: {o}")
        else:
            by_uid.pop(match)
    problems.extend(f"{k}: fazla satır" for k in by_uid)
    return "; ".join(problems) or None


def written_komsu():
    """kentos-uzlasti-komsu.gpkg (tests/all/reconcile.rs, rest: layer, both layers in one call) by OGR: each row of
    each table as the reconcile says."""
    path = DIR / WRITTEN_K
    if not path.exists():
        return f"{WRITTEN_K} yok: KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all reconcile"
    k, b, _ = bound2("x.gpkg", K_EDITS, B_EDITS)
    _, after, uid_after, layer_of = solve2(k, b, rest="layer")
    problems = []
    for table, lid in (("komsu", KOMSU), ("bina", BINA)):
        got = {r["uid"]: obj(r) for r in rows(WRITTEN_K, table).values()}
        want = {uid_after[s]: o for s, o in after.items() if layer_of[s] == lid and uid_after[s] is not None}
        for u, o in want.items():
            g = got.pop(u, None)
            if g is None:
                problems.append(f"{table} {u}: satır yok")
            elif not same_object(o, g):
                problems.append(f"{table} {u}: {g} ≠ {o}")
        problems.extend(f"{table} {u}: fazla satır" for u in got)
    return "; ".join(problems) or None


def main():
    if "--files" in sys.argv:
        write_files()
        return 0
    if "--written" in sys.argv:
        for name, check in ((WRITTEN, written), (WRITTEN_K, written_komsu)):
            why = check()
            if why:
                print(why)
                return 1
            print(f"{name}: uzlaştırmanın yazdığı satırlar doğru")
        return 0
    data = {"format": "kentos.command-cases", "version": 1, "command": "cad.sources.reconcile",
            "commandVersion": 1, "setup": bs.SETUP, "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil")
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum güncel")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
