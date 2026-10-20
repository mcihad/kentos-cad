#!/usr/bin/env python3
"""The shared cases of cad.meshes.describe and cad.meshes.query (docs/adr/0552; plan/gis.md CBS-32).

    uv run --no-project --with mpmath --with numpy python3 scripts/fixtures/mesh_cases.py --files   # writes fixtures/meshes/v1/files
    uv run --no-project --with mpmath --with numpy python3 scripts/fixtures/mesh_cases.py           # writes the cases
    uv run --no-project --with mpmath --with numpy python3 scripts/fixtures/mesh_cases.py --check   # compares

No KentOS code is used. The files are fixtures/multidim/v1/files (ugrid.nc and the broken ones, written by
multidim_cases.py with netcdf_classic.py) and two written here with the same classic NetCDF writer from the UGRID 1.0
conventions: quad-static.nc (a geographic mesh of two quadrilaterals and a triangle, start index 0, a node dataset and a
face dataset without a time dimension) and layered.nc (two quadrilaterals, start index 1, a node dataset on (time,
layer, node) with a missing value, a face dataset on (time, face)). The reference's own reading of a mesh (the topology
variable's coordinates and connectivity, the datasets and their vectors, masks and dimensions, the time axes' moments
and labels) follows the ADR (§4) and ADR 0552; the face a place lies in and the values are those of multidim_cases.py
(the faces as triangles, the lowest whose closed area holds the place, found exactly with fractions; node values
interpolated exactly, a vector's magnitude at 50 digits). The face's area is exact (fractions), the edges are the faces'
edges each once, the mean edge the hypotenuse of each at 50 digits.
"""
import base64
import json
import math
import sys
from fractions import Fraction
from pathlib import Path

import mpmath
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import multidim_cases as md  # noqa: E402
import netcdf_classic as nc  # noqa: E402

mpmath.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
MD_FILES = ROOT / "fixtures/multidim/v1/files"
FILES = ROOT / "fixtures/meshes/v1/files"
OUT = ROOT / "fixtures/commands/v1"
ANY = "$any"
WITHIN = 1e-9
HOUR = 3_600_000


def fx(path):
    return "$fixture:" + str(path.relative_to(ROOT / "fixtures"))


# ── The two files written here ─────────────────────────────────────────

def static_file():
    lon = [32.0, 32.1, 32.2, 32.0, 32.1, 32.2, 32.1]
    lat = [36.0, 36.0, 36.0, 36.1, 36.1, 36.1, 36.2]
    faces = [[0, 1, 4, 3], [1, 2, 5, 4], [3, 4, 6, -1]]
    vs = [
        md.var("mesh", [], "int", [0], {"cf_role": "mesh_topology", "topology_dimension": ("int", [2]),
                                        "node_coordinates": "lon lat", "face_node_connectivity": "faces"}),
        md.var("lon", ["node"], "double", lon, {"standard_name": "longitude", "units": "degrees_east"}),
        md.var("lat", ["node"], "double", lat, {"standard_name": "latitude", "units": "degrees_north"}),
        md.var("faces", ["face", "nmax"], "int", [v for f in faces for v in f],
               {"cf_role": "face_node_connectivity", "start_index": ("int", [0]), "_FillValue": ("int", [-1])}),
        md.var("elev", ["node"], "float", [100.0 + 10.5 * k for k in range(7)],
               {"mesh": "mesh", "location": "node", "long_name": "Yükseklik", "units": "m"}),
        md.var("zone", ["face"], "int", [1, 2, 3], {"mesh": "mesh", "location": "face", "long_name": "Bölge"}),
    ]
    return nc.write(None, 2, [("node", 7), ("face", 3), ("nmax", 4)], {"Conventions": "CF-1.8 UGRID-1.0"}, vs)


LAYERED_NODES = [(500000.0, 4420000.0), (500010.0, 4420000.0), (500010.0, 4420010.0), (500000.0, 4420010.0),
                 (500020.0, 4420000.0), (500020.0, 4420010.0)]
LAYERED_FACES = [[1, 2, 3, 4], [2, 5, 6, 3]]


def layered_temp(t, l, k):
    return 10.0 * t + 3.0 * l + 0.25 * k


def layered_file():
    n, f = len(LAYERED_NODES), len(LAYERED_FACES)
    temp = []
    for t in range(2):
        for l in range(2):
            for k in range(n):
                temp.append(md.F_FILL if (t, l, k) == (1, 1, 4) else layered_temp(t, l, k))
    wind = [2.0 + t + 0.5 * q for t in range(2) for q in range(f)]
    vs = [
        md.var("mesh", [], "int", [0], {"cf_role": "mesh_topology", "topology_dimension": ("int", [2]),
                                        "node_coordinates": "node_x node_y", "face_node_connectivity": "faces",
                                        "grid_mapping": "crs"}),
        md.var("crs", [], "int", [0], {"epsg_code": "EPSG:5254"}),
        md.var("node_x", ["node"], "double", [p[0] for p in LAYERED_NODES], {"standard_name": "projection_x_coordinate"}),
        md.var("node_y", ["node"], "double", [p[1] for p in LAYERED_NODES], {"standard_name": "projection_y_coordinate"}),
        md.var("faces", ["face", "nmax"], "int", [v for fc in LAYERED_FACES for v in fc],
               {"cf_role": "face_node_connectivity", "start_index": ("int", [1])}),
        md.var("time", ["time"], "double", [0.0, 6.0], {"units": "hours since 2025-01-01 00:00:00"}),
        md.var("layer", ["layer"], "double", [0.0, 10.0], {"units": "m"}),
        md.var("temp", ["time", "layer", "node"], "float", temp,
               {"mesh": "mesh", "location": "node", "long_name": "Sıcaklık", "units": "degC",
                "_FillValue": ("float", [md.F_FILL])}),
        md.var("wind", ["time", "face"], "float", wind, {"mesh": "mesh", "location": "face", "units": "m/s"}),
    ]
    return nc.write(None, 2, [("node", n), ("face", f), ("nmax", 4), ("time", 2), ("layer", 2)],
                    {"Conventions": "CF-1.8 UGRID-1.0"}, vs)


NEW_FILES = {"quad-static.nc": static_file, "layered.nc": layered_file}


# ── The reference's own reading of a mesh ──────────────────────────────

class Ref:
    """A UGRID file as the ADR reads it: nodes, faces (from 0, in the file's order), datasets, masks."""

    def __init__(self, path):
        self.path = path
        self.data = path.read_bytes()
        h = self.h = nc.read_header(self.data)
        self.kinds = md.attr_kinds(self.data)
        topo = next(v for v in h.vars if (md.text_attr(v, "cf_role") or "").strip() == "mesh_topology")
        self.name = topo["name"]
        nx, ny = md.text_attr(topo, "node_coordinates").split()[:2]
        fv = next(v for v in h.vars if v["name"] == md.text_attr(topo, "face_node_connectivity"))
        self.node_dim = next(v for v in h.vars if v["name"] == nx)["dims"][0]
        self.face_dim = fv["dims"][0]
        self.hidden = {nx, ny, fv["name"]}
        xs = [float(v) for v in nc.read_var(self.data, h, nx)]
        ys = [float(v) for v in nc.read_var(self.data, h, ny)]
        self.nodes = list(zip(xs, ys))
        start = int(fv["attrs"].get("start_index", [0])[0])
        fills = list(fv["attrs"].get("_FillValue", []))
        most = h.dims[fv["dims"][1]][1]
        raw = [float(v) for v in nc.read_var(self.data, h, fv["name"])]
        self.faces = []
        for f in range(h.dims[self.face_dim][1]):
            row = raw[f * most:(f + 1) * most]
            self.faces.append([int(v) - start for v in row if v not in fills and v >= start])
        self.tris = md.triangles(self.nodes, self.faces)
        self.datasets = self.find_datasets()

    def find_datasets(self):
        h = self.h
        found = []
        for v in h.vars:
            if v["kind"] == 2 or not v["dims"]:
                continue
            if (md.text_attr(v, "mesh") or "").strip() != self.name:
                continue
            loc = (md.text_attr(v, "location") or "").strip()
            if loc not in ("node", "face"):
                continue
            own = self.node_dim if loc == "node" else self.face_dim
            if v["dims"][-1] != own or own in v["dims"][:-1]:
                continue
            c = md.coordinate_of(h, own)
            if v["name"] in self.hidden or (c is not None and c["name"] == v["name"]) or md.text_attr(v, "cf_role"):
                continue
            mask = None
            m = (md.text_attr(v, "kentos_mask") or "").strip()
            mv = next((x for x in h.vars if x["name"] == m), None) if m else None
            if mv is not None and mv["dims"][-1] == self.face_dim and len(mv["dims"]) == len(v["dims"]):
                mask = mv["name"]
            found.append({"variable": v["name"], "vector": None, "location": loc, "dims": v["dims"][:-1], "mask": mask})
        masks = [d["mask"] for d in found if d["mask"]]
        found = [d for d in found if d["variable"] not in masks]
        return self.pair(found)

    @staticmethod
    def components(a, b):
        if (a, b) == ("u", "v"):
            return True
        if len(a) != len(b):
            return False
        diff = [(p, q) for p, q in zip(a, b) if p != q]
        return len(diff) == 1 and (diff[0][0].lower(), diff[0][1].lower()) == ("x", "y")

    def pair(self, allv):
        n = len(allv)

        def paired(i, j):
            a, b = allv[i], allv[j]
            return (a["location"] == b["location"] and a["dims"] == b["dims"] and a["mask"] == b["mask"]
                    and (self.components(a["variable"], b["variable"]) or self.components(b["variable"], a["variable"])))

        mate = []
        for i in range(n):
            ms = [j for j in range(n) if j != i and paired(i, j)]
            mate.append(ms[0] if len(ms) == 1 else None)
        out, taken = [], [False] * n
        for i in range(n):
            if taken[i]:
                continue
            taken[i] = True
            j = mate[i]
            if j is not None and not taken[j] and mate[j] == i:
                taken[j] = True
                x, y = (i, j) if self.components(allv[i]["variable"], allv[j]["variable"]) else (j, i)
                d = dict(allv[x])
                d["vector"] = allv[y]["variable"]
                out.append(d)
            else:
                out.append(allv[i])
        return out

    def var(self, name):
        return next(v for v in self.h.vars if v["name"] == name)

    def slice_dims(self, d):
        return [md.slice_dim(self.h, self.data, k) for k in d["dims"]]

    def units(self, name):
        return (md.text_attr(self.var(name), "units") or "").strip() or None

    def long_name(self, name):
        return (md.text_attr(self.var(name), "long_name") or "").strip() or None

    def places(self, name, idx):
        """A variable's values at one slice (`idx`, a step for each leading dimension): None where missing."""
        v = self.var(name)
        raw = nc.read_var(self.data, self.h, name)
        sizes = [self.h.dims[k][1] for k in v["dims"]]
        stride = 1
        strides = []
        for s in reversed(sizes):
            strides.append(stride)
            stride *= s
        strides.reverse()
        base = sum(i * st for i, st in zip(idx, strides[:-1]))
        rule = md.rule_of(v, self.kinds)
        out = []
        for k in range(sizes[-1]):
            u = md.unpacked(rule, float(raw[base + k]))
            out.append(None if (isinstance(u, float) and math.isnan(u)) else u)
        return out

    def mask_at(self, d, idx):
        if d["mask"] is None:
            return None
        v = self.var(d["mask"])
        raw = nc.read_var(self.data, self.h, d["mask"])
        sizes = [self.h.dims[k][1] for k in v["dims"]]
        stride, strides = 1, []
        for s in reversed(sizes):
            strides.append(stride)
            stride *= s
        strides.reverse()
        base = sum(i * st for i, st in zip(idx, strides[:-1]))
        return [float(raw[base + k]) != 0 for k in range(sizes[-1])]

    def value(self, d, idx, x, y):
        """The dataset's value at a place for the slice `idx`, a float or None; and the face the place lies in."""
        ti = md.locate(self.nodes, self.tris, x, y)
        if ti is None:
            return None, None
        t, face, convex = self.tris[ti]
        data = {"location": d["location"], "x": self.places(d["variable"], idx), "mask": self.mask_at(d, idx)}
        if d["vector"]:
            data["y"] = self.places(d["vector"], idx)
        v = md.mesh_value(self.nodes, self.tris, data, x, y)
        f = md.to_float(v)
        return (None if math.isnan(f) else f), face

    def face_of(self, x, y):
        ti = md.locate(self.nodes, self.tris, x, y)
        return None if ti is None else self.tris[ti][1]

    def face_area(self, f):
        p = [tuple(Fraction(c) for c in self.nodes[k]) for k in self.faces[f]]
        twice = sum(md.orient(p[0], p[i], p[i + 1]) for i in range(1, len(p) - 1))
        return float(abs(twice) / 2)

    def edges(self):
        e = set()
        for f in self.faces:
            for i in range(len(f)):
                a, b = f[i], f[(i + 1) % len(f)]
                e.add((min(a, b), max(a, b)))
        return sorted(e)

    def area(self):
        total = Fraction(0)
        for f in range(len(self.faces)):
            p = [tuple(Fraction(c) for c in self.nodes[k]) for k in self.faces[f]]
            total += abs(sum(md.orient(p[0], p[i], p[i + 1]) for i in range(1, len(p) - 1))) / 2
        return float(total)

    def mean_edge(self):
        es = self.edges()
        s = mpmath.mpf(0)
        for a, b in es:
            s += mpmath.sqrt((mpmath.mpf(self.nodes[b][0]) - mpmath.mpf(self.nodes[a][0])) ** 2
                             + (mpmath.mpf(self.nodes[b][1]) - mpmath.mpf(self.nodes[a][1])) ** 2)
        return float(s / len(es))

    def epsg(self):
        topo = self.var(self.name)
        for v in (topo, ):
            gm = (md.text_attr(v, "grid_mapping") or "").strip()
            crs = next((x for x in self.h.vars if x["name"] == gm), None) if gm else None
            if crs is not None:
                code = (md.text_attr(crs, "epsg_code") or "").upper().replace("EPSG:", "")
                if code.isdigit():
                    return int(code)
        xv = self.var(md.text_attr(topo, "node_coordinates").split()[0])
        if md.axis_of_var(xv) == "Lon":
            return 4326
        return None

    def geographic(self):
        xv = self.var(md.text_attr(self.var(self.name), "node_coordinates").split()[0])
        return md.axis_of_var(xv) == "Lon"

    def bbox(self):
        xs = [p[0] for p in self.nodes]
        ys = [p[1] for p in self.nodes]
        return [min(xs), min(ys), max(xs), max(ys)]


# ── The drawing ────────────────────────────────────────────────────────

REFS = {
    "ugrid": Ref(MD_FILES / "ugrid.nc"),
    "sms": Ref(MD_FILES / "sms-start.nc"),
    "static": None,
    "layered": None,
}
STYLE = {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Viridis"}


def refs():
    if REFS["static"] is None:
        REFS["static"] = Ref(FILES / "quad-static.nc")
        REFS["layered"] = Ref(FILES / "layered.nc")
    return REFS


def dims_entry(ref, d, index):
    dims = []
    for s in ref.slice_dims(d):
        e = {"name": s["name"], "index": index.pop(0), "values": s["values"]}
        if s["time"]:
            e["time"] = True
        if s.get("units"):
            e["units"] = s["units"]
        dims.append(e)
    return dims


def raster(id_, layer_id, ref, file, variable, index, vector=None, mesh=None, asset=None, srid=5254):
    d = next(x for x in ref.datasets if x["variable"] == variable)
    dataset = {"variable": variable, "mesh": mesh if mesh is not None else ref.name, "dims": dims_entry(ref, d, list(index))}
    if vector:
        dataset["vector"] = vector
    e = {"kind": "raster", "id": id_, "layerId": layer_id, "attrs": {}, "affine": md.MESH_AFFINE, "width": md.MESH_SIZE[0],
         "height": md.MESH_SIZE[1], "bands": 1, "sample": "f32", "srid": srid, "style": STYLE, "dataset": dataset}
    if asset:
        e["asset"] = asset
    else:
        e["file"] = file
    return e


ASSET = "mesh-0552055205520552"


def drawing():
    """Ids: 1 ugrid.nc depth (slice 1), 2 ugrid.nc vector ucx/ucy (slice 2), 3 quad-static elev, 4 quad-static zone,
    5 layered temp (time 1, layer 1), 6 layered wind (time 0), 7 sms-start.nc (2DM and DAT), 8 embedded ugrid.nc level
    (slice 0), 9 hidden ugrid.nc, 10 file not found, 11 broken, 12 NetCDF-4, 13 GRIB, 14 a grid raster (no mesh),
    15 a line, 16 a topology the file does not have."""
    r = refs()
    u, s, q, l, m = r["ugrid"], r["sms"], r["static"], r["layered"], r["sms"]
    ents = [
        raster(1, "mesh", u, fx(MD_FILES / "ugrid.nc"), "depth", [1]),
        raster(2, "mesh", u, fx(MD_FILES / "ugrid.nc"), "ucx", [2], vector="ucy"),
        raster(3, "mesh", q, fx(FILES / "quad-static.nc"), "elev", [], srid=4326),
        raster(4, "mesh", q, fx(FILES / "quad-static.nc"), "zone", [], srid=4326),
        raster(5, "mesh", l, fx(FILES / "layered.nc"), "temp", [1, 1]),
        raster(6, "mesh", l, fx(FILES / "layered.nc"), "wind", [0]),
        raster(7, "mesh", s, fx(MD_FILES / "sms-start.nc"), s.datasets[0]["variable"], [0] * len(s.datasets[0]["dims"])),
        raster(8, "mesh", u, None, "level", [0], asset=ASSET),
        raster(9, "gizli", u, fx(MD_FILES / "ugrid.nc"), "depth", [0]),
        raster(10, "mesh", u, fx(MD_FILES / "yok.nc"), "depth", [0]),
        raster(11, "mesh", u, fx(MD_FILES / "bad-header.nc"), "depth", [0]),
        raster(12, "mesh", u, fx(MD_FILES / "bad-hdf5.nc"), "depth", [0]),
        raster(13, "mesh", u, fx(MD_FILES / "bad-grib.nc"), "depth", [0]),
        {"kind": "raster", "id": 14, "layerId": "mesh", "attrs": {}, "affine": [500000.0, 10.0, 0.0, 4420030.0, 0.0, -10.0],
         "width": 4, "height": 3, "bands": 1, "sample": "f32", "srid": 5254, "file": fx(MD_FILES / "grid-cdf2.nc"),
         "style": STYLE, "dataset": {"variable": "dem", "dims": []}},
        {"kind": "line", "id": 15, "layerId": "mesh", "attrs": {}, "a": {"x": 0, "y": 0}, "b": {"x": 1, "y": 1}},
        raster(16, "mesh", u, fx(MD_FILES / "ugrid.nc"), "depth", [0], mesh="yok"),
    ]
    data = base64.b64encode((MD_FILES / "ugrid.nc").read_bytes()).decode()

    def layer(i, name, visible=True):
        return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": False, "expanded": True,
                "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    return {
        "format": "kentos.document", "version": 1, "name": "Meshlerin bilgisi",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis"},
        "origin": {"x": 500000.0, "y": 4420000.0},
        "layers": [layer("mesh", "Mesh"), layer("gizli", "Gizli", visible=False)],
        "activeLayer": "mesh", "entities": ents,
        "styles": {"items": [{"kind": "asset", "id": ASSET, "name": "ugrid.nc",
                              "data": f"data:application/octet-stream;base64,{data}"}], "categories": []},
    }


def entity(id_):
    return drawing()["entities"][id_ - 1]


# ── Describe ───────────────────────────────────────────────────────────

def dimension(s):
    out = {"name": s["name"], "count": len(s["values"]), "time": s["time"]}
    if s.get("units"):
        out["units"] = s["units"]
    out.update({"first": s["values"][0], "last": s["values"][-1], "firstLabel": s["labels"][0], "lastLabel": s["labels"][-1]})
    return out


def shown_of(e):
    d = e["dataset"]
    steps = []
    for dim in d["dims"]:
        label = md.dim_labels([dim["values"][dim["index"]]], dim.get("time", False), dim.get("units"))[0]
        steps.append({"name": dim["name"], "index": dim["index"], "label": label})
    out = {"variable": d["variable"]}
    if d.get("vector"):
        out["vector"] = d["vector"]
    if steps:
        out["steps"] = steps
    return out


def described(id_, ref):
    e = entity(id_)
    out = {"uid": f"$uidOf:{id_}", "layer": e["layerId"], "mesh": e["dataset"]["mesh"], "read": True, "srid": e["srid"],
           "nodes": len(ref.nodes), "faces": len(ref.faces), "edges": len(ref.edges()),
           "triangles": len(ref.tris), "bbox": ref.bbox(), "area": ref.area(), "meanEdge": ref.mean_edge(),
           "geographic": ref.geographic(), "shown": shown_of(e)}
    if "file" in e:
        out["file"] = e["file"]
    if "asset" in e:
        out["asset"] = e["asset"]
    if ref.epsg() is not None:
        out["epsg"] = ref.epsg()
    sets = []
    for d in ref.datasets:
        s = {"variable": d["variable"], "location": d["location"]}
        if d["vector"]:
            s["vector"] = d["vector"]
        if ref.long_name(d["variable"]):
            s["longName"] = ref.long_name(d["variable"])
        if ref.units(d["variable"]):
            s["units"] = ref.units(d["variable"])
        dims = [dimension(x) for x in ref.slice_dims(d)]
        if dims:
            s["dims"] = dims
        sets.append(s)
    out["datasets"] = sets
    return out


def unread(id_, why):
    e = entity(id_)
    out = {"uid": f"$uidOf:{id_}", "layer": e["layerId"], "mesh": e["dataset"]["mesh"], "read": False, "why": why,
           "nodes": 0, "faces": 0, "edges": 0, "triangles": 0, "geographic": False, "srid": e["srid"], "datasets": [],
           "shown": shown_of(e)}
    if "file" in e:
        out["file"] = e["file"]
    return out


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def step(inp, result, op="execute", within=True):
    s = {"op": op, "input": inp, "result": result, "expect": {"revision": "same"}}
    if within:
        s["within"] = WITHIN
    return s


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def answer(meshes):
    return {"meshes": meshes, "revision": "$current"}


MESH_WARN = "Meshin dosyası okunamadı: {}."


def describe_cases():
    r = refs()
    out = []
    names = {1: ("ugrid", "ugrid.nc: derinlik, düzey, vektör, maske, üç zaman adımı"),
             2: ("ugrid", "ugrid.nc: gösterilen veri seti vektör (ucx, ucy)"),
             3: ("static", "quad-static.nc: coğrafi mesh, zamansız veri setleri"),
             4: ("static", "quad-static.nc: yüz veri seti gösterilen"),
             5: ("layered", "layered.nc: zaman ve katman boyutları, gösterilen adım ikisinde de 1"),
             6: ("layered", "layered.nc: yüzdeki veri seti, yalnız zaman boyutu"),
             7: ("sms", "2DM ve DAT'tan dönüşen UGRID: Taban kotu ve DAT veri setleri"),
             8: ("ugrid", "gömülü mesh kitaplıktan okunur")}
    for id_, (key, title) in names.items():
        out.append(case(title, step({"uids": [f"$uidOf:{id_}"]}, done(answer([described(id_, r[key])])))))
    out.append(case("gizli katmandaki mesh adıyla sorulunca okunur", step(
        {"uids": ["$uidOf:9"]}, done(answer([{**described(9, r["ugrid"]), "layer": "gizli"}])))))
    out.append(case("dosyası olmayan mesh okunmaz, söylenir", step(
        {"uids": ["$uidOf:10"]}, done(answer([unread(10, ANY)]), [{"code": "mesh_source_unreadable", "message": ANY,
                                                                   "path": "meshes[0]"}]))))
    for id_, what in ((11, "bozuk dosya"), (12, "NetCDF-4 (HDF5) dosyası okunmaz"), (13, "GRIB dosyası okunmaz")):
        out.append(case(f"{what}: neden söylenir, öbürleri okunur", step(
            {"uids": [f"$uidOf:{id_}", "$uidOf:1"]},
            done(answer([unread(id_, ANY), described(1, r["ugrid"])]),
                 [{"code": "mesh_source_unreadable", "message": ANY, "path": "meshes[0]"}]))))
    out.append(case("dosyanın olmayan bir ağı: neden söylenir", step(
        {"uids": ["$uidOf:16"]}, done(answer([unread(16, "NetCDF'te “yok” ağı yok")]),
                                      [{"code": "mesh_source_unreadable",
                                        "message": "Meshin dosyası okunamadı: NetCDF'te “yok” ağı yok.",
                                        "path": "meshes[0]"}]))))
    out.append(case("iki mesh: kimliklerin sırasıyla", step(
        {"uids": ["$uidOf:5", "$uidOf:3"]}, done(answer([described(5, r["layered"]), described(3, r["static"])])))))
    out.append(case("aynı mesh iki kez verilse bir kez söylenir", step(
        {"uids": ["$uidOf:3", "$uidOf:3"]}, done(answer([described(3, r["static"])])))))
    everything = []
    warns = []
    keys = {1: "ugrid", 2: "ugrid", 3: "static", 4: "static", 5: "layered", 6: "layered", 7: "sms", 8: "ugrid", 9: "ugrid"}
    for id_ in range(1, 17):
        e = entity(id_)
        if e["kind"] != "raster" or "mesh" not in e["dataset"] and e["dataset"].get("mesh") is None:
            continue
        if id_ in keys:
            d = described(id_, r[keys[id_]])
            if id_ == 9:
                d["layer"] = "gizli"
            everything.append(d)
        else:
            why = "NetCDF'te “yok” ağı yok" if id_ == 16 else ANY
            everything.append(unread(id_, why))
            warns.append({"code": "mesh_source_unreadable",
                          "message": MESH_WARN.format(why) if id_ == 16 else ANY, "path": f"meshes[{len(everything) - 1}]"})
    out.append(case("uids verilmezse çizimin bütün meshleri sırasıyla (gizli katmandakiler dahil)", step(
        {}, done(answer(everything), warns))))
    out.append(case("plan aynı cevabı yazmadan verir", step(
        {"uids": ["$uidOf:3"]}, done(answer([described(3, r["static"])])), op="plan")))
    out.append(case("validate yalnız denetler", step(
        {"uids": ["$uidOf:3"]}, {"status": "completed", "output": None, "warnings": []}, op="validate")))
    uid_text = ("geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.")
    gone = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4eff"
    not_mesh = ("Verilen nesne bir mesh rasteri değil; bir meshin (UGRID veri setini gösteren rasterin) kimliğini verin.")
    out += [
        case("geçersiz kimlik", step({"uids": ["mesh"]}, failed("invalid_uid", f"“mesh” {uid_text}", "uids[0]"))),
        case("çizimde olmayan nesne", step({"uids": [gone]}, failed(
            "entity_not_found",
            f"“{gone}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
            "uids[0]"))),
        case("mesh olmayan raster (CF ızgarası)", step({"uids": ["$uidOf:14"]}, failed("not_a_mesh", not_mesh, "uids[0]"))),
        case("raster olmayan nesne", step({"uids": ["$uidOf:15"]}, failed("not_a_mesh", not_mesh, "uids[0]"))),
        case("ikinci kimlik mesh değil", step({"uids": ["$uidOf:1", "$uidOf:15"]}, failed("not_a_mesh", not_mesh, "uids[1]"))),
    ]
    return out


# ── Query ──────────────────────────────────────────────────────────────

def moment(ref, d, step_):
    return ref.slice_dims(d)[next(i for i, s in enumerate(ref.slice_dims(d)) if s["time"])]["values"][step_]


def value_entry(ref, d, shown_idx, at, time=None, series=False, shown_moment=None):
    """One dataset's answer at a place: the step at the moment, the value, the series; and the values read."""
    dims = ref.slice_dims(d)
    tk = next((i for i, s in enumerate(dims) if s["time"]), None)
    entry = {"variable": d["variable"], "location": d["location"]}
    if d["vector"]:
        entry["vector"] = d["vector"]
    if ref.units(d["variable"]):
        entry["units"] = ref.units(d["variable"])
    per = (3 if d["location"] == "node" else 1) * (2 if d["vector"] else 1) + (1 if d["mask"] else 0)
    idx = [shown_idx.get(s["name"], 0) for s in dims]
    if tk is None:
        v, _ = ref.value(d, idx, *at)
        if v is not None:
            entry["value"] = v
        return entry, per
    times = dims[tk]["values"]
    m = time if time is not None else shown_moment
    st = 0 if m is None else sum(1 for t in times if t <= m) - 1
    st = None if st < 0 else st
    steps_read = len(times) if series else (0 if st is None else 1)
    if st is not None:
        idx2 = list(idx)
        idx2[tk] = st
        v, _ = ref.value(d, idx2, *at)
        entry["step"] = st
        entry["time"] = times[st]
        if v is not None:
            entry["value"] = v
    if series:
        labels = md.dim_labels(times, True, None)
        rows = []
        for i, t in enumerate(times):
            idx2 = list(idx)
            idx2[tk] = i
            v, _ = ref.value(d, idx2, *at)
            row = {"step": i, "time": t, "label": labels[i]}
            if v is not None:
                row["value"] = v
            rows.append(row)
        entry["series"] = rows
    return entry, steps_read * per


def queried(id_, key, at, time=None, series=False, variables=None, op="execute"):
    """The step of a place in mesh `id_` (reference `key`): the face and the values of the asked datasets."""
    ref = refs()[key]
    e = entity(id_)
    inp = {"at": {"x": at[0], "y": at[1]}, "uids": [f"$uidOf:{id_}"]}
    if time is not None:
        inp["time"] = time
    if series:
        inp["series"] = True
    if variables:
        inp["variables"] = variables
    out = {"revision": "$current", "read": 0}
    face = ref.face_of(*at)
    if face is not None:
        out["face"] = {"mesh": f"$uidOf:{id_}", "topology": ref.name, "face": face, "nodes": ref.faces[face],
                       "area": ref.face_area(face)}
        shown = {s["name"]: s["index"] for s in e["dataset"]["dims"]}
        sm = None
        for s in e["dataset"]["dims"]:
            if s.get("time"):
                sm = s["values"][s["index"]]
        chosen = ref.datasets
        if variables:
            chosen = [next(d for d in ref.datasets if d["variable"] == v) for v in variables]
        vals, read = [], 0
        for d in chosen:
            # The raster's slice applies to its own dataset only; the others stand at their first step of each dimension.
            own = shown if d["variable"] == e["dataset"]["variable"] else {}
            entry, n = value_entry(ref, d, own, at, time, series, sm)
            vals.append(entry)
            read += n
        out["values"] = vals
        out["read"] = read
    return step(inp, done(out), op=op)


def query_cases():
    out = []
    q = lambda *a, **k: queried(*a, **k)  # noqa: E731
    # ugrid.nc: the faces (6), the depth shown at step 1; the concave face (4) only answers its face dataset.
    pts = {"f0": (500010.0, 4420010.0), "f1": (500035.0, 4420005.0), "f2": (500025.0, 4420010.0), "f3": (500010.0, 4420030.0),
           "f5": (500030.0, 4420036.0)}
    u = refs()["ugrid"]
    for name, at in pts.items():
        out.append(case(f"ugrid.nc: {name} yüzünde bütün veri setleri", q(1, "ugrid", at)))
    concave = next((x for x in [(500036.0, 4420024.0), (500037.0, 4420022.0), (500038.0, 4420030.0)] if u.face_of(*x) == 4), None)
    assert concave is not None
    out.append(case("ugrid.nc: içbükey yüzde yalnız yüz veri seti sorulur", q(1, "ugrid", concave, variables=["level"])))
    out.append(case("ugrid.nc: ortak kenardaki yer en küçük üçgenin yüzüne gider", q(1, "ugrid", (500010.0, 4420020.0))))
    out.append(case("ugrid.nc: ortak düğümdeki yer", q(1, "ugrid", (500020.0, 4420020.0))))
    out.append(case("ugrid.nc: mesh dışındaki yer: yüz yok", q(1, "ugrid", (400000.0, 4420010.0))))
    out.append(case("ugrid.nc: eksik düğüm değeri değeri yok eder (adım 1'de 4. düğüm)", q(1, "ugrid", (500010.0, 4420030.0), variables=["depth"])))
    out.append(case("ugrid.nc: kapalı yüzde (maske 0) derinlik yok, düzey var", q(1, "ugrid", pts["f1"], variables=["depth", "level"])))
    out.append(case("ugrid.nc: yalnız vektör: büyüklüğü", q(2, "ugrid", pts["f0"], variables=["ucx"])))
    m0 = moment(u, u.datasets[0], 0)
    for label, t in (("ilk adımdan önce: değer yok", m0 - 1.0), ("tam ilk adımda", m0), ("iki adımın arasında: öncekinin", m0 + 900_000.0),
                     ("tam son adımda", moment(u, u.datasets[0], 2)), ("son adımdan sonra: sonuncusu", moment(u, u.datasets[0], 2) + 7_200_000.0)):
        out.append(case(f"ugrid.nc: an {label}", q(1, "ugrid", pts["f0"], time=t, variables=["depth", "level"])))
    out.append(case("ugrid.nc: seri: her adımın değeri", q(1, "ugrid", pts["f0"], series=True, variables=["depth", "ucx"])))
    out.append(case("ugrid.nc: ilk adımdan önceki an ve seri", q(1, "ugrid", pts["f0"], time=m0 - 1.0, series=True, variables=["level"])))
    out.append(case("gömülü mesh kitaplıktan okunur", q(8, "ugrid", pts["f0"], variables=["level"])))
    # quad-static.nc: geographic, without a time dimension.
    s = refs()["static"]
    out.append(case("quad-static.nc: dörtgenin içinde yükseklik ve bölge", q(3, "static", (32.05, 36.05))))
    out.append(case("quad-static.nc: üçgenin içinde", q(3, "static", (32.09, 36.11))))
    out.append(case("quad-static.nc: seri istenirse zamansız veri setinin serisi yok", q(3, "static", (32.15, 36.05), series=True)))
    out.append(case("quad-static.nc: an verilse de zamansız veri seti aynı", q(3, "static", (32.15, 36.05), time=1.0e12)))
    # layered.nc: the shown time and layer, then the moment, the series.
    l = refs()["layered"]
    out.append(case("layered.nc: gösterilen zaman ve katman adımında", q(5, "layered", (500005.0, 4420005.0))))
    out.append(case("layered.nc: eksik değerin üçgeninde sıcaklık yok", q(5, "layered", (500015.0, 4420005.0), variables=["temp"])))
    out.append(case("layered.nc: gösterilen veri seti dışındakiler katmanın ilk değerinde", q(6, "layered", (500005.0, 4420005.0))))
    t0 = moment(l, l.datasets[0], 0)
    out.append(case("layered.nc: sıcaklık, anı ilk adımda", q(5, "layered", (500005.0, 4420005.0), time=t0 + HOUR, variables=["temp"])))
    out.append(case("layered.nc: sıcaklığın serisi", q(5, "layered", (500015.0, 4420005.0), series=True, variables=["temp", "wind"])))
    # 2DM and DAT converted to UGRID.
    sm = refs()["sms"]
    pts_sms = [(5.0, 5.0), (12.0, 15.0), (25.0, 3.0), (35.0, 15.0)]
    for at in pts_sms[:2]:
        out.append(case(f"2DM'den dönüşen mesh: {at}", q(7, "sms", at)))
    # Several meshes.
    both = [f"$uidOf:{i}" for i in (1, 3)]
    out.append(case("iki mesh: ilkinde yer yoksa ikincisine bakılır", step(
        {"at": {"x": 32.05, "y": 36.05}, "uids": both}, queried_in(3, "static", (32.05, 36.05)))))
    out.append(case("uids verilmezse görünen meshler: gizli katmandaki bakılmaz", step(
        {"at": {"x": pts["f0"][0], "y": pts["f0"][1]}}, queried_visible(pts["f0"]))))
    out.append(case("okunamayan mesh uyarılır, sonrakine bakılır", step(
        {"at": {"x": pts["f0"][0], "y": pts["f0"][1]}, "uids": ["$uidOf:10", "$uidOf:1"], "variables": ["level"]},
        queried_in(1, "ugrid", pts["f0"], variables=["level"], warnings=[
            {"code": "mesh_source_unreadable", "message": ANY, "path": "meshes[0]"}]))))
    out.append(case("plan aynı cevabı yazmadan verir", q(1, "ugrid", pts["f0"], op="plan")))
    out.append(case("validate yalnız denetler", step(
        {"at": {"x": 0.0, "y": 0.0}}, {"status": "completed", "output": None, "warnings": []}, op="validate")))
    uid_text = ("geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.")
    gone = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4eff"
    out += [
        case("bilinmeyen veri seti", step(
            {"at": {"x": pts["f0"][0], "y": pts["f0"][1]}, "uids": ["$uidOf:1"], "variables": ["depth", "yok"]}, failed(
                "mesh_variable_not_found", "“yok” veri seti meshte yok; var olanlar: depth, level, ucx.",
                "variables[1]"))),
        case("geçersiz kimlik", step({"at": {"x": 0.0, "y": 0.0}, "uids": ["mesh"]}, failed(
            "invalid_uid", f"“mesh” {uid_text}", "uids[0]"))),
        case("çizimde olmayan nesne", step({"at": {"x": 0.0, "y": 0.0}, "uids": [gone]}, failed(
            "entity_not_found",
            f"“{gone}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
            "uids[0]"))),
        case("mesh olmayan nesne", step({"at": {"x": 0.0, "y": 0.0}, "uids": ["$uidOf:14"]}, failed(
            "not_a_mesh",
            "Verilen nesne bir mesh rasteri değil; bir meshin (UGRID veri setini gösteren rasterin) kimliğini verin.",
            "uids[0]"))),
    ]
    return out


def queried_in(id_, key, at, variables=None, warnings=()):
    s = queried(id_, key, at, variables=variables)
    result = s["result"]
    result["warnings"] = list(warnings)
    return result


def queried_visible(at):
    """No uids: the visible meshes in the drawing's order; the first (1) holds the place and answers."""
    return queried_in(1, "ugrid", at)


NOTE = ("ADR 0552. Bağımsız başvuru scripts/fixtures/mesh_cases.py: dosyalar netcdf_classic.py'nin yazdığı klasik "
        "NetCDF'ler, ağın okunuşu UGRID 1.0 kurallarından, yüz ve değerler multidim_cases.py'nin kesirli aramasından; "
        "KentOS kodu yok.")


def build(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": NOTE, "setup": drawing(), "cases": cases}


def main():
    if "--files" in sys.argv:
        FILES.mkdir(parents=True, exist_ok=True)
        for name, make in NEW_FILES.items():
            (FILES / name).write_bytes(make())
            print(f"yazıldı: {(FILES / name).relative_to(ROOT)}")
        return 0
    for name, make in NEW_FILES.items():
        if "--check" in sys.argv and (not (FILES / name).exists() or (FILES / name).read_bytes() != make()):
            print(f"{name} eski: betiği --files ile çalıştırın", file=sys.stderr)
            return 1
    files = {
        "cad.meshes.describe": ("Meshin bilgisi", describe_cases),
        "cad.meshes.query": ("Mesh sor", query_cases),
    }
    bad = 0
    for command, (title, make) in files.items():
        out = OUT / f"{command}.json"
        text = json.dumps(build(command, title, make()), ensure_ascii=False, indent=1, allow_nan=False) + "\n"
        if "--check" in sys.argv:
            if not out.exists() or out.read_text("utf-8") != text:
                print(f"{out.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
                bad += 1
            continue
        out.write_text(text, "utf-8")
        print(f"yazıldı: {out.relative_to(ROOT)}")
    if "--check" in sys.argv and not bad:
        print("Mesh bilgisi ve sorgusu durumları tutarlı")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
