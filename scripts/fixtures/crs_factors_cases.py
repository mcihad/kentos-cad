#!/usr/bin/env python3
"""The shared cases of `cad.crs.factors` (plan/map-cad.md MAP-02, docs/adr/1231), against PROJ, without KentOS code.

    python3 scripts/fixtures/crs_factors_cases.py           # writes fixtures/commands/v1/cad.crs.factors.json
    python3 scripts/fixtures/crs_factors_cases.py --check   # writes nothing; compares

Needs pyproj (PROJ); without it here the script runs itself again under QGIS's Python on macOS (crs_chain_cases.py's
way). The rules are ADR 1231's, written here by hand:

- the projection's scale at a point is PROJ's (`Proj.get_factors`: the parallel scale, equal to the meridian scale on a
  conformal projection), at the point's latitude and longitude PROJ inverts it to;
- the height's factor along the east is N / (N + h): N the ellipsoid's prime vertical radius a / √(1 − e² sin²φ) (the
  radius of curvature at azimuth 90°), h the given height, else the project's mean ellipsoidal height;
- the combined factor is their product; without a height only the scale is given;
- a project on its own coordinates has no system (`no_system`); a geographic system and the Pseudo-Mercator have no one
  scale at a point (`no_point_scale`); a point PROJ cannot invert gets no factors (warning `point_beyond_system`).

ADR 2900's rules, here by hand too:

- each point may have its own ellipsoidal height (`heights`), or the drawing's height (`elevations`) made ellipsoidal by
  the project's heights: as it is when ellipsoidal, plus N when orthometric, N read from the geoid model's GTX file
  (fixtures/geoid/v1/tg.gtx, its SHA-256 the model's) by this script's own bilinear reading between the four nodes; a
  point outside the grid or by a node without a value gets no height (warning `points_outside`);
- a line between two points (`lines`, their indexes): its scale by Simpson's rule on PROJ's scales at its ends and its
  plane middle, (k₁ + 4kₘ + k₂)/6; its height factor R/(R + h) with R Euler's radius MN/(N cos²α + M sin²α) at the
  geodesic's middle in its azimuth there (PROJ's geodesic, `Geod.inv` and `Geod.fwd` to half its length), h its ends'
  heights' mean (or the one height); their product.
"""

import hashlib
import json
import math
import os
import struct
import sys
from pathlib import Path

try:
    import pyproj  # noqa: F401
except ImportError:
    qgis = sorted(Path("/Applications").glob("QGIS*.app/Contents"))
    if not qgis or os.environ.get("KENTOS_QGIS_PYTHON"):
        raise
    res = qgis[-1] / "Resources"
    env = dict(os.environ, KENTOS_QGIS_PYTHON="1", PYTHONHOME=str(res),
               PYTHONPATH=os.pathsep.join(str(res / p) for p in ("python3.12", "python3.12/site-packages",
                                                                 "python3.12/lib-dynload")))
    exe = qgis[-1] / "MacOS" / "python3.12"
    os.execve(str(exe), [str(exe), *sys.argv], env)

sys.dont_write_bytecode = True
from pyproj import CRS, Geod, Proj, Transformer  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.crs.factors.json"
GTX = ROOT / "fixtures" / "geoid" / "v1" / "tg.gtx"
TG = {"name": "TG-20 (sentetik)", "sha256": hashlib.sha256(GTX.read_bytes()).hexdigest()}

# The systems' names as a sentence names them (the registry's titles; metadata, not a computation).
TITLES = {
    5254: "TUREF / TM30 (EPSG:5254)",
    5255: "TUREF / TM33 (EPSG:5255)",
    5256: "TUREF / TM36 (EPSG:5256)",
    32635: "WGS 84 / UTM 35N (EPSG:32635)",
    23035: "ED50 / UTM 35N (EPSG:23035)",
    4326: "WGS 84 (EPSG:4326)",
    3857: "WGS 84 / Pseudo-Mercator (EPSG:3857)",
}


def settings(srid, height=None, reduce=None, heights=None):
    s = {"srid": srid, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"}
    if heights is not None:
        s["heights"] = heights
    survey = {}
    if height is not None:
        survey["groundHeight"] = height
    if reduce is not None:
        survey["reduceToGrid"] = reduce
    if survey:
        s["survey"] = survey
    return s


def setup(srid, height=None, reduce=None, heights=None):
    layer = {"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True,
             "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    return {"format": "kentos.document", "version": 1, "name": "Ölçek çarpanları",
            "settings": settings(srid, height, reduce, heights), "origin": {"x": 500000, "y": 4400000},
            "layers": [layer], "activeLayer": "0", "entities": [], "styles": {"items": [], "categories": []}}


def read_gtx(path):
    """A GTX file: the south-west node's latitude and longitude, the steps (big-endian f64), rows and columns (i32),
    then float32 values row by row from the south; −88.8888 is no value."""
    b = path.read_bytes()
    south, west, dlat, dlon = struct.unpack(">dddd", b[:32])
    rows, cols = struct.unpack(">ii", b[32:40])
    values = struct.unpack(f">{rows * cols}f", b[40:40 + 4 * rows * cols])
    return south, west, dlat, dlon, rows, cols, values


GRID = read_gtx(GTX)


def undulation(lon, lat):
    """N between the four nodes round the point, bilinear; None outside the grid or by a node without a value."""
    south, west, dlat, dlon, rows, cols, values = GRID
    fx, fy = (lon - west) / dlon, (lat - south) / dlat
    if not (0 <= fx <= cols - 1 and 0 <= fy <= rows - 1):
        return None
    i, j = min(int(math.floor(fx)), cols - 2), min(int(math.floor(fy)), rows - 2)
    u, v = fx - i, fy - j
    a, b2, c, d = values[j * cols + i], values[j * cols + i + 1], values[(j + 1) * cols + i], values[(j + 1) * cols + i + 1]
    if any(abs(x + 88.8888) < 1e-3 for x in (a, b2, c, d)):
        return None
    low, high = a + (b2 - a) * u, c + (d - c) * u
    return low + (high - low) * v


def lonlat(srid, x, y):
    crs = CRS.from_epsg(srid)
    return Transformer.from_crs(crs, crs.geodetic_crs, always_xy=True).transform(x, y, errcheck=False)


def scale_at(srid, x, y):
    lon, lat = lonlat(srid, x, y)
    return Proj(CRS.from_epsg(srid)).get_factors(lon, lat, errcheck=False).parallel_scale


def line_factors(srid, a, b, h):
    """A line's Simpson scale and, with a height, R/(R + h) at the geodesic's middle in its azimuth there."""
    m = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    k = (scale_at(srid, *a) + 4 * scale_at(srid, *m) + scale_at(srid, *b)) / 6
    out = {"scale": k}
    if h is None:
        return out
    e = CRS.from_epsg(srid).ellipsoid
    ea, f = e.semi_major_metre, 1.0 / e.inverse_flattening
    geod = Geod(a=ea, f=f)
    (lon1, lat1), (lon2, lat2) = lonlat(srid, *a), lonlat(srid, *b)
    az, _back, dist = geod.inv(lon1, lat1, lon2, lat2)
    _lonm, latm, back = geod.fwd(lon1, lat1, az, dist / 2)
    alpha = math.radians((back + 180.0) % 360.0)
    e2 = f * (2.0 - f)
    w2 = 1.0 - e2 * math.sin(math.radians(latm)) ** 2
    mm, nn = ea * (1.0 - e2) / (w2 * math.sqrt(w2)), ea / math.sqrt(w2)
    r = mm * nn / (nn * math.cos(alpha) ** 2 + mm * math.sin(alpha) ** 2)
    out["heightFactor"] = r / (r + h)
    out["combined"] = k * out["heightFactor"]
    out["height"] = h
    return out


def factors(srid, x, y, h):
    """PROJ's point scale at (x, y) of the system, N/(N + h) on its ellipsoid along the east, their product."""
    crs = CRS.from_epsg(srid)
    lonlat = Transformer.from_crs(crs, crs.geodetic_crs, always_xy=True)
    lon, lat = lonlat.transform(x, y, errcheck=False)
    if not (math.isfinite(lon) and math.isfinite(lat)) or abs(lat) > 90:
        return None
    k = Proj(crs).get_factors(lon, lat, errcheck=False).parallel_scale
    if not math.isfinite(k):
        return None
    out = {"scale": k}
    if h is not None:
        e = crs.ellipsoid
        a, f = e.semi_major_metre, 1.0 / e.inverse_flattening
        e2 = f * (2.0 - f)
        s = math.sin(math.radians(lat))
        n = a / math.sqrt(1.0 - e2 * s * s)
        out["heightFactor"] = n / (n + h)
        out["combined"] = k * out["heightFactor"]
    return out


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


SAME = {"revision": "same", "canUndo": False}


def read(srid, points, project_height=None, height=None, reduce=None, heights=None, elevations=None, lines=None,
         project_heights=None):
    single = height if height is not None else project_height
    own, ns, geoid, warnings = None, [None] * len(points), None, []
    if heights is not None:
        own = list(heights)
    elif elevations is not None:
        own = []
        orthometric = project_heights["kind"] == "orthometric"
        if orthometric:
            geoid = project_heights["geoid"]["name"]
        for i, ((x, y), z) in enumerate(zip(points, elevations)):
            n = 0.0
            if orthometric:
                n = undulation(*lonlat(srid, x, y))
                if n is None:
                    own.append(None)
                    warnings.append({"code": "points_outside", "path": f"elevations[{i}]",
                                     "message": f"{i + 1}. nokta “{geoid}” geoit modelinin ızgarasının dışında; kotu "
                                                "elipsoit yüksekliğine çevrilmedi, yükseklik çarpanı verilmedi."})
                    continue
                ns[i] = n
            own.append(z + n)
    rows = []
    for i, (x, y) in enumerate(points):
        h = own[i] if own is not None else single
        f = factors(srid, x, y, h)
        if f is None:
            rows.append({})
            warnings.append({"code": "point_beyond_system", "path": f"points[{i}]",
                             "message": f"{i + 1}. nokta {TITLES[srid]} sisteminin ulaştığı yerin dışında; çarpanları verilmedi."})
            continue
        if own is not None and own[i] is not None:
            f["height"] = own[i]
        if ns[i] is not None:
            f["undulation"] = ns[i]
        rows.append(f)
    out = {"system": TITLES[srid], "reducesToGrid": bool(reduce) and project_height is not None, "points": rows,
           "revision": "$current"}
    if own is None and single is not None:
        out["height"] = single
    if lines:
        out["lines"] = []
        for a, b in lines:
            if own is not None:
                h = None if own[a] is None or own[b] is None else (own[a] + own[b]) / 2.0
            else:
                h = single
            out["lines"].append(line_factors(srid, points[a], points[b], h))
    if geoid is not None:
        out["geoid"] = geoid
    return done(out, warnings)


def case(name, srid, points, project_height=None, height=None, reduce=None, op="execute", note=None, heights=None,
         elevations=None, lines=None, project_heights=None):
    input_ = {"points": [[float(x), float(y)] for x, y in points]}
    if height is not None:
        input_["height"] = height
    if heights is not None:
        input_["heights"] = heights
    if elevations is not None:
        input_["elevations"] = elevations
    if lines is not None:
        input_["lines"] = lines
    result = read(srid, points, project_height, height, reduce, heights, elevations, lines, project_heights)
    if op == "validate":
        result = done(None, result["warnings"])
    c = {"name": name}
    if note:
        c["note"] = note
    c["setup"] = setup(srid, project_height, reduce, project_heights)
    c["steps"] = [{"op": op, "input": input_, "result": result, "expect": SAME, "within": 1e-9}]
    return c


def refused(name, srid, input_, code, message, path, project_heights=None):
    return {"name": name, "setup": setup(srid, heights=project_heights), "steps": [
        {"op": "execute", "input": input_, "result": failed(code, message, path), "expect": SAME}]}


def tm36(lat, lon):
    return Transformer.from_crs("EPSG:4326", "EPSG:5256", always_xy=True).transform(lon, lat)


ORTHO = {"kind": "orthometric", "verticalSrid": 5775, "geoid": TG}
ELLIPSOIDAL = {"kind": "ellipsoidal"}


N0 = 4_540_000.0


def cases():
    out = [
        case("TM30'un orta meridyeninde ölçek 1, 850 m'de yükseklik çarpanı", 5254, [(500_000, N0)], 850.0),
        case("Orta meridyenden 50 km doğuda: +30,8 ppm", 5254, [(550_000, N0)], 850.0),
        case("Batıda simetrik: aynı ölçek", 5254, [(450_000, N0)], 850.0),
        case("Üç nokta birlikte, sırayla", 5254, [(500_000, N0), (525_000, N0 + 10_000), (575_000, N0 - 20_000)], 850.0),
        case("Güneyde (Hakkari enlemi) 100 km doğuda", 5254, [(600_000, 4_150_000)], 1800.0),
        case("Verilen yükseklik projeninkinin yerine geçer: 0 m'de çarpan 1", 5254, [(550_000, N0)], 850.0, height=0.0),
        case("Deniz altında (−200 m) çarpan 1'den büyük", 5254, [(550_000, N0)], None, height=-200.0),
        case("Sınırda yükseklik 9000 m", 5254, [(550_000, N0)], None, height=9000.0),
        case("Yükseklik yoksa yalnız projeksiyon ölçeği", 5254, [(550_000, N0)]),
        case("Uzunlukları projeksiyona indiren proje: reducesToGrid", 5254, [(550_000, N0)], 850.0, reduce=True),
        case("İndirme açık ama yükseklik yok: indirmez", 5254, [(550_000, N0)], None, reduce=True),
        case("TM33'te aynı yer", 5255, [(550_000, N0)], 850.0),
        case("UTM 35N'de orta meridyende k₀ = 0,9996", 32635, [(500_000, N0)], 850.0),
        case("UTM 35N'de orta meridyenden 200 km uzakta ölçek 1'i geçer", 32635, [(700_000, N0)], 850.0),
        case("ED50 / UTM 35N: Hayford elipsoidinin N'si", 23035, [(550_000, N0)], 850.0,
             note="Yükseklik çarpanı sistemin elipsoidiyle (International 1924) hesaplanır."),
        case("Sistemin ulaşmadığı nokta (50 000 km doğu): çarpansız, uyarıyla; öbürü verilir", 5254,
             [(550_000, N0), (50_000_000, N0)], 850.0),
        case("Plan aynı cevabı verir, yazmaz", 5254, [(550_000, N0)], 850.0, op="plan"),
        case("Doğrulama yazmaz", 5254, [(550_000, N0)], 850.0, op="validate"),
        refused("Yerel proje: sistem yok", 0, {"points": [[0.0, 0.0]]}, "no_system",
                "Proje kendi koordinatlarında (yerel); projeksiyon ölçeği yok. Projeye bir koordinat sistemi atayın.",
                "points"),
        refused("Coğrafi sistem: bir noktanın tek ölçeği yok", 4326, {"points": [[30.0, 40.0]]}, "no_point_scale",
                f"{TITLES[4326]}: bu sistemde bir noktanın tek ölçeği yok (coğrafi sistem, Pseudo-Mercator ya da afinle "
                "bağlı yerel sistem).", "points"),
        refused("Pseudo-Mercator: bir noktanın tek ölçeği yok", 3857, {"points": [[3_339_584.7, 4_865_942.3]]},
                "no_point_scale",
                f"{TITLES[3857]}: bu sistemde bir noktanın tek ölçeği yok (coğrafi sistem, Pseudo-Mercator ya da afinle "
                "bağlı yerel sistem).", "points"),
        refused("Nokta yok", 5254, {"points": []}, "invalid_input", "1 ile 10000 arasında nokta verin; 0 verildi.",
                "points"),
        refused("Yükseklik aralık dışında", 5254, {"points": [[550_000.0, N0]], "height": -600.0}, "invalid_input",
                "Yükseklik -600 m olamaz; −500 ile 9000 m arasında bir ortalama elipsoit yüksekliği verin.", "height"),
        refused("Yükseklik 9000 m'yi aşıyor", 5254, {"points": [[550_000.0, N0]], "height": 9000.5}, "invalid_input",
                "Yükseklik 9000.5 m olamaz; −500 ile 9000 m arasında bir ortalama elipsoit yüksekliği verin.", "height"),
    ]
    out += own_heights()
    return out


def own_heights():
    """ADR 2900: each point's own height, the drawing's heights through the geoid, lines."""
    a, b, c = (550_000.0, N0), (551_200.0, N0 + 800.0), (549_100.0, N0 + 1500.0)
    kayseri, sivas = tm36(38.7312, 35.4787), tm36(39.7477, 37.0179)
    near = (kayseri[0] + 900.0, kayseri[1] + 400.0)
    kars = tm36(42.95, 45.8)
    both = "Yüksekliği bir yoldan verin: height (bütün noktalara bir yükseklik), heights (noktaların elipsoit " \
           "yükseklikleri) ya da elevations (çizimin kotları)."
    return [
        case("Her noktanın kendi elipsoit yüksekliği: çarpanı o yükseklikte", 5254, [a, b], 850.0,
             heights=[850.0, 1450.0]),
        case("Çizgiler: uçlarının yüksekliklerinin ortalamasında, doğrultusundaki Euler yarıçapıyla", 5254, [a, b, c],
             850.0, heights=[850.0, 1450.0, 610.25], lines=[[0, 1], [1, 2], [2, 0]]),
        case("Çizgi projenin ortalama yüksekliğinde", 5254, [a, b], 850.0, lines=[[0, 1]]),
        case("Çizgi verilen tek yükseklikte", 5254, [a, b], 850.0, height=1200.0, lines=[[1, 0]]),
        case("Yüksekliksiz çizgi: yalnız ölçek", 5254, [a, b], lines=[[0, 1]]),
        case("Kotlar elipsoit yüksekliği: olduğu gibi", 5254, [a, b], 850.0, elevations=[900.0, 1320.5],
             lines=[[0, 1]], project_heights=ELLIPSOIDAL),
        case("Ortometrik kotlar ve TG-20 geoidi: h = H + N, çizgi uçlarının ortalamasında", 5256,
             [kayseri, near, sivas], 1100.0, elevations=[1100.0, 1062.4, 1350.5], lines=[[0, 1], [0, 2]],
             project_heights=ORTHO,
             note="N betiğin GTX okumasıyla, dört düğüm arasında çift doğrusal (fixtures/geoid/v1/tg.gtx)."),
        case("Geoidin ızgarası dışındaki nokta: kotu çevrilmez, uyarıyla; çizgisi yalnız ölçekle", 5256,
             [kayseri, kars], 1100.0, elevations=[1100.0, 1750.0], lines=[[0, 1]], project_heights=ORTHO),
        refused("Yükseklik iki yoldan: height ve heights", 5254,
                {"points": [list(a)], "height": 850.0, "heights": [850.0]}, "invalid_input", both, "heights"),
        refused("Yükseklik iki yoldan: heights ve elevations", 5254,
                {"points": [list(a)], "heights": [850.0], "elevations": [800.0]}, "invalid_input", both, "elevations"),
        refused("Nokta başına bir yükseklik değil", 5254, {"points": [list(a), list(b)], "heights": [850.0]},
                "invalid_input", "Her nokta için bir yükseklik verin: 2 nokta, 1 yükseklik.", "heights"),
        refused("Noktanın yüksekliği aralık dışında", 5254, {"points": [list(a)], "heights": [9100.0]},
                "invalid_input", "Yükseklik 9100 m olamaz; −500 ile 9000 m arasında bir elipsoit yüksekliği verin.",
                "heights[0]"),
        refused("Nokta başına bir kot değil", 5254, {"points": [list(a)], "elevations": [800.0, 900.0]},
                "invalid_input", "Her nokta için bir kot verin: 1 nokta, 2 kot.", "elevations"),
        refused("Çizginin ucu nokta değil", 5254, {"points": [list(a), list(b)], "lines": [[0, 2]]}, "invalid_input",
                "Çizginin ucu 2 bir nokta değil: 2 nokta verildi, sıraları 0'dan 1'e.", "lines[0]"),
        refused("Çizginin iki ucu aynı yerde", 5254, {"points": [list(a), list(a)], "lines": [[0, 1]]},
                "invalid_input", "Çizginin iki ucu aynı yerde; uzunluğu yok.", "lines[0]"),
        refused("Kotların ne olduğu söylenmemiş", 5254, {"points": [list(a)], "elevations": [800.0]}, "heights_unknown",
                "Projenin kotlarının ne olduğu (elipsoit ya da ortometrik yükseklik) söylenmemiş: kotlardan elipsoit "
                "yüksekliği çıkarılmaz. Kotları Proje ayarları › Kotlar'da söyleyin.", "elevations"),
        refused("Ortometrik kot, geoit modeli yok", 5256, {"points": [list(kayseri)], "elevations": [1100.0]},
                "geoid_required",
                "Projenin kotları ortometrik ama geoit modeli (SHA-256'sıyla) yok: kotlar elipsoit yüksekliğine "
                "çevrilemez. Geoit modelini Proje ayarları › Kotlar'da seçin.", "elevations",
                project_heights={"kind": "orthometric", "verticalSrid": 5775}),
        refused("Geoit modelinin ızgarası bu cihazda yok", 5256, {"points": [list(kayseri)], "elevations": [1100.0]},
                "geoid_missing",
                "“TG-99 (yok)” geoit modelinin ızgarası (aaaaaaaaaaaa…) bu cihazın ızgara kitaplığında yok. Izgarayı "
                "Proje ayarları › Izgaralar'a ekleyin; kotlar sıfır geoit ayrımıyla çevrilmez.", "elevations",
                project_heights={"kind": "orthometric", "verticalSrid": 5775,
                                 "geoid": {"name": "TG-99 (yok)", "sha256": "a" * 64}}),
        refused("Kotun elipsoit yüksekliği aralık dışında", 5254, {"points": [list(a)], "elevations": [9500.0]},
                "invalid_input", "Kotun elipsoit yüksekliği 9500 m çıkıyor; −500 ile 9000 m arasında olmalı.",
                "elevations[0]", project_heights=ELLIPSOIDAL),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.crs.factors", "commandVersion": 1,
           "title": "Ölçek çarpanları",
           "note": "ADR 1231. Bağımsız başvuru scripts/fixtures/crs_factors_cases.py: projeksiyon ölçeği PROJ'un "
                   "get_factors'ı (pyproj " + pyproj.__version__ + ", PROJ " + pyproj.proj_version_str + "), yükseklik "
                   "çarpanı N/(N + h) elle; KentOS kodu yok.",
           "setup": setup(5254, 850.0), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        # The PROJ version in the note may differ on another machine; the numbers are compared within 1e-12.
        old = json.loads(OUT.read_text("utf-8")) if OUT.exists() else None
        if old is None or old["cases"] != json.loads(text)["cases"]:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            return 1
        print("Ölçek çarpanı ortak durumları güncel.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
