#!/usr/bin/env python3
"""Mesh ve çok boyutlu verinin resimlerinin çizimi (docs/adr/0243): paylaşılan vadide saatlik yağışın NetCDF ızgarası
(batıdan doğuya geçen bir fırtına hücresi) ve derenin taşkınının UGRID ağı (yarım saatlik adımlarla mansaba inen dalga:
su derinliği, hız vektörü, taban kotu), dereyi kesen kesit çizgisi ve üç gözlem noktası.

    python3 scripts/fixtures/multidim_scene.py          # fixtures/interaction/v1/multidim.kcad'i ve dosyalarını yazar
    python3 scripts/fixtures/multidim_scene.py --check  # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz: dosyalar başvurunun NetCDF yazıcısıyla (scripts/fixtures/netcdf_classic.py) yazılır.
- Yağış: 48 × 32 hücre (16 m × 16,2 m), y artan (okuyucu çevirir), 12 saat; yoğunluk = 0,4 + 18·exp(−r²/2σ²) mm/saat,
  hücrenin merkezi saatte 56 m doğuya, σ 120 m; 0,1'e yuvarlanmış. Sistem `grid_mapping` ile EPSG:5256.
- Taşkın: derenin ekseni y = 4 420 340 + 60·sin(1,5π·(x − x₀)/768); düğümler eksenden −120 … 120 m (15 m'de bir),
  x'te 24 m'de bir; her dörtgen iki üçgen (köşegen sırayla). Taban kotu 850 − 0,03·(x − x₀) + 0,002·v²; eksendeki
  derinlik D = 0,4 + 2,2·exp(−((x − (x₀ + 70·t))/220)²), düğümde D − 0,0035·v², 0,01 m'den sığ düğüm değersiz (kuru).
  Hız dere boyunca, büyüklüğü 1,2·√derinlik.
"""

import datetime
import json
import math
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import netcdf_classic as nc  # noqa: E402

OUT = ROOT / "fixtures/interaction/v1/multidim.kcad"
DIR = ROOT / "fixtures/interaction/v1/multidim"
X0, YTOP = 487200.0, 4420600.0
WIDTH, HEIGHT = 768.0, 518.4
SRID = 5256
# NetCDF's default fill of a 32-bit float (NC_FILL_FLOAT), as the file holds it.
F_FILL = struct.unpack(">f", struct.pack(">f", 9.9692099683868690e36))[0]
START = "2024-05-01 06:00:00"


def var(name, dims, kind, values, attrs=None):
    return nc.Var(name, dims, kind, values, attrs or {})


def rain():
    nx, ny, steps = 48, 32, 12
    dx, dy = WIDTH / nx, HEIGHT / ny
    xs = [X0 + dx * (i + 0.5) for i in range(nx)]
    ys = [YTOP - HEIGHT + dy * (j + 0.5) for j in range(ny)]
    values = []
    for t in range(steps):
        cx, cy = X0 + 80 + 56 * t, 4420360.0
        for y in ys:
            for x in xs:
                r2 = (x - cx) ** 2 + (y - cy) ** 2
                values.append(round(0.4 + 18 * math.exp(-r2 / (2 * 120.0 ** 2)), 1))
    vs = [
        var("crs", [], "int", [0], {"epsg_code": f"EPSG:{SRID}"}),
        var("x", ["x"], "double", xs, {"standard_name": "projection_x_coordinate", "units": "m"}),
        var("y", ["y"], "double", ys, {"standard_name": "projection_y_coordinate", "units": "m"}),
        var("time", ["time"], "double", [float(t) for t in range(steps)], {"standard_name": "time", "units": f"hours since {START}"}),
        var("yagis", ["time", "y", "x"], "float", values, {"long_name": "Yağış", "units": "mm/saat", "grid_mapping": "crs"}),
    ]
    data = nc.write(None, 2, [("x", nx), ("y", ny), ("time", steps)], {"Conventions": "CF-1.8", "title": "Vadinin yağışı"}, vs)
    affine = [X0, dx, 0.0, YTOP, 0.0, -dy]
    return data, affine, (nx, ny), [moment_ms(3600 * t) for t in range(steps)]


def moment_ms(seconds):
    """Milliseconds since 1970 of `START` plus `seconds`."""
    base = datetime.datetime(2024, 5, 1, 6, 0, 0, tzinfo=datetime.timezone.utc)
    return float((base - datetime.datetime(1970, 1, 1, tzinfo=datetime.timezone.utc)).total_seconds() * 1000 + seconds * 1000)


def river(x):
    return 4420340.0 + 60.0 * math.sin(1.5 * math.pi * (x - X0) / WIDTH)


def river_slope(x):
    return 60.0 * 1.5 * math.pi / WIDTH * math.cos(1.5 * math.pi * (x - X0) / WIDTH)


def flood():
    cols, rows, steps = 33, 17, 12
    nodes, offsets = [], []
    for i in range(cols):
        x = X0 + 24.0 * i
        for j in range(rows):
            v = -120.0 + 15.0 * j
            nodes.append((x, round(river(x) + v, 3)))
            offsets.append(v)
    faces = []
    for i in range(cols - 1):
        for j in range(rows - 1):
            a, b = i * rows + j, (i + 1) * rows + j
            c, d = b + 1, a + 1
            if (i + j) % 2 == 0:
                faces += [[a, b, c], [a, c, d]]
            else:
                faces += [[a, b, d], [b, c, d]]
    bed = [850.0 - 0.03 * (x - X0) + 0.002 * v * v for (x, _), v in zip(nodes, offsets)]
    depth, ux, uy = [], [], []
    for t in range(steps):
        for (x, _), v in zip(nodes, offsets):
            dc = 0.4 + 2.2 * math.exp(-(((x - (X0 + 70.0 * t)) / 220.0) ** 2))
            dd = dc - 0.0035 * v * v
            if dd < 0.01:
                depth.append(F_FILL)
                ux.append(F_FILL)
                uy.append(F_FILL)
                continue
            depth.append(round(dd, 3))
            s = 1.2 * math.sqrt(dd)
            k = river_slope(x)
            n = math.hypot(1.0, k)
            ux.append(round(s / n, 3))
            uy.append(round(s * k / n, 3))
    most = 3
    vs = [
        var("mesh", [], "int", [0], {"cf_role": "mesh_topology", "topology_dimension": ("int", [2]),
                                     "node_coordinates": "node_x node_y", "face_node_connectivity": "faces", "grid_mapping": "crs"}),
        var("crs", [], "int", [0], {"epsg_code": f"EPSG:{SRID}"}),
        var("node_x", ["node"], "double", [p[0] for p in nodes], {"standard_name": "projection_x_coordinate", "units": "m"}),
        var("node_y", ["node"], "double", [p[1] for p in nodes], {"standard_name": "projection_y_coordinate", "units": "m"}),
        var("faces", ["face", "nmax"], "int", [k for f in faces for k in f], {"cf_role": "face_node_connectivity", "start_index": ("int", [0])}),
        var("time", ["time"], "double", [30.0 * t for t in range(steps)], {"standard_name": "time", "units": f"minutes since {START}"}),
        var("taban", ["node"], "double", bed, {"mesh": "mesh", "location": "node", "long_name": "Taban kotu", "units": "m"}),
        var("derinlik", ["time", "node"], "float", depth, {"mesh": "mesh", "location": "node", "long_name": "Su derinliği", "units": "m",
                                                           "_FillValue": ("float", [F_FILL])}),
        var("hiz_x", ["time", "node"], "float", ux, {"mesh": "mesh", "location": "node", "long_name": "Hız (x)", "units": "m/s",
                                                     "_FillValue": ("float", [F_FILL])}),
        var("hiz_y", ["time", "node"], "float", uy, {"mesh": "mesh", "location": "node", "long_name": "Hız (y)", "units": "m/s",
                                                     "_FillValue": ("float", [F_FILL])}),
    ]
    dims = [("node", len(nodes)), ("face", len(faces)), ("nmax", most), ("time", steps)]
    data = nc.write(None, 2, dims, {"Conventions": "CF-1.8 UGRID-1.0", "title": "Derenin taşkını"}, vs)
    # The virtual grid (docs/adr/0243 §5): an eighth of the mean face's side, 1-2-5 down, over the mesh's box.
    area = 0.0
    for f in faces:
        (x1, y1), (x2, y2), (x3, y3) = (nodes[k] for k in f)
        area += abs((x2 - x1) * (y3 - y1) - (y2 - y1) * (x3 - x1)) / 2
    mean = math.sqrt(area / len(faces)) / 8.0
    e = math.floor(math.log10(mean))
    m = mean / 10.0 ** e
    cell = (5.0 if m >= 5 else 2.0 if m >= 2 else 1.0) * 10.0 ** e
    xs, ys = [p[0] for p in nodes], [p[1] for p in nodes]
    gx0 = math.floor(min(xs) / cell) * cell
    gy0 = math.ceil(max(ys) / cell) * cell
    w = max(1, math.ceil((max(xs) - gx0) / cell))
    h = max(1, math.ceil((gy0 - min(ys)) / cell))
    return data, [gx0, cell, 0.0, gy0, 0.0, -cell], (w, h), [moment_ms(1800 * t) for t in range(steps)]


def drawing(rain_grid, flood_grid):
    def layer(id, name, color):
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    entities = []

    def add(e, lid, attrs=None):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs or {}})

    r_affine, (rw, rh), r_times = rain_grid
    f_affine, (fw, fh), f_times = flood_grid
    add({"kind": "raster", "affine": f_affine, "width": fw, "height": fh, "bands": 1, "sample": "f32", "file": "multidim/taskin.nc",
         "srid": SRID, "style": {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Viridis", "edges": "#2B3440"},
         "dataset": {"variable": "derinlik", "mesh": "mesh",
                     "dims": [{"name": "time", "index": 9, "values": f_times, "time": True}], "followTime": True}}, "taskin")
    add({"kind": "raster", "affine": r_affine, "width": rw, "height": rh, "bands": 1, "sample": "f32", "file": "multidim/yagis.nc",
         "srid": SRID, "style": {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Mavi-kırmızı"},
         "opacity": 0.6,
         "dataset": {"variable": "yagis", "dims": [{"name": "time", "index": 4, "values": r_times, "time": True}], "followTime": True}},
        "yagis")
    # A section across the river and one along it; three gauges by its bank.
    xs = 487608.0
    add({"kind": "polyline", "pts": [{"x": xs, "y": river(xs) - 150}, {"x": xs, "y": river(xs) + 150}]}, "kesit", {"ad": "Kesit 1"})
    along = [X0 + 24.0 * i for i in range(2, 31, 4)]
    add({"kind": "polyline", "pts": [{"x": x, "y": round(river(x), 3)} for x in along]}, "kesit", {"ad": "Eksen"})
    for name, x in (("Köprü", 487320.0), ("Değirmen", 487560.0), ("Mansap", 487848.0)):
        add({"kind": "point", "p": {"x": x, "y": round(river(x) + 6.0, 3)}}, "istasyonlar", {"ad": name})
    # The flood over the rain: its dry places show the rain under them.
    layers = [
        layer("istasyonlar", "İstasyonlar", "#C62828"),
        layer("kesit", "Kesitler", "#37474F"),
        layer("taskin", "Taşkın", "#8D6E63"),
        layer("yagis", "Yağış", "#8D6E63"),
    ]
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Mesh ve çok boyutlu veri",
        "settings": {"srid": SRID, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000.0,
                     "workspace": "gis"},
        "origin": {"x": 487500.0, "y": 4420300.0},
        "layers": layers,
        "activeLayer": "kesit",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def main():
    check = "--check" in sys.argv[1:]
    rain_bytes, r_affine, r_size, r_times = rain()
    flood_bytes, f_affine, f_size, f_times = flood()
    text = json.dumps(drawing((r_affine, r_size, r_times), (f_affine, f_size, f_times)), ensure_ascii=False, indent=1) + "\n"
    files = {DIR / "yagis.nc": rain_bytes, DIR / "taskin.nc": flood_bytes}
    problems = []
    if check:
        for path, data in files.items():
            if not path.exists() or path.read_bytes() != data:
                problems.append(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil")
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            problems.append(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil")
    else:
        DIR.mkdir(parents=True, exist_ok=True)
        for path, data in files.items():
            path.write_bytes(data)
        OUT.write_text(text, "utf-8")
    if problems:
        print("\n".join(problems), file=sys.stderr)
        sys.exit(1)
    print(f"{OUT.relative_to(ROOT)}: yağış {r_size[0]} × {r_size[1]} hücre, taşkın ağı {f_size[0]} × {f_size[1]} hücrelik sanal ızgarada")


if __name__ == "__main__":
    main()
