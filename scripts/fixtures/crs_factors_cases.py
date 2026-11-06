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
"""

import json
import math
import os
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
from pyproj import CRS, Proj, Transformer  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.crs.factors.json"

# The systems' names as a sentence names them (the registry's titles; metadata, not a computation).
TITLES = {
    5254: "TUREF / TM30 (EPSG:5254)",
    5255: "TUREF / TM33 (EPSG:5255)",
    32635: "WGS 84 / UTM 35N (EPSG:32635)",
    23035: "ED50 / UTM 35N (EPSG:23035)",
    4326: "WGS 84 (EPSG:4326)",
    3857: "WGS 84 / Pseudo-Mercator (EPSG:3857)",
}


def settings(srid, height=None, reduce=None):
    s = {"srid": srid, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"}
    survey = {}
    if height is not None:
        survey["groundHeight"] = height
    if reduce is not None:
        survey["reduceToGrid"] = reduce
    if survey:
        s["survey"] = survey
    return s


def setup(srid, height=None, reduce=None):
    layer = {"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True,
             "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    return {"format": "kentos.document", "version": 1, "name": "Ölçek çarpanları",
            "settings": settings(srid, height, reduce), "origin": {"x": 500000, "y": 4400000}, "layers": [layer],
            "activeLayer": "0", "entities": [], "styles": {"items": [], "categories": []}}


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


def read(srid, points, project_height=None, height=None, reduce=None):
    h = height if height is not None else project_height
    rows, warnings = [], []
    for i, (x, y) in enumerate(points):
        f = factors(srid, x, y, h)
        if f is None:
            rows.append({})
            warnings.append({"code": "point_beyond_system", "path": f"points[{i}]",
                             "message": f"{i + 1}. nokta {TITLES[srid]} sisteminin ulaştığı yerin dışında; çarpanları verilmedi."})
        else:
            rows.append(f)
    out = {"system": TITLES[srid], "reducesToGrid": bool(reduce) and project_height is not None, "points": rows,
           "revision": "$current"}
    if h is not None:
        out["height"] = h
    return done(out, warnings)


def case(name, srid, points, project_height=None, height=None, reduce=None, op="execute", note=None):
    input_ = {"points": [[float(x), float(y)] for x, y in points]}
    if height is not None:
        input_["height"] = height
    result = read(srid, points, project_height, height, reduce)
    if op == "validate":
        result = done(None, result["warnings"])
    c = {"name": name}
    if note:
        c["note"] = note
    c["setup"] = setup(srid, project_height, reduce)
    c["steps"] = [{"op": op, "input": input_, "result": result, "expect": SAME, "within": 1e-9}]
    return c


def refused(name, srid, input_, code, message, path):
    return {"name": name, "setup": setup(srid), "steps": [
        {"op": "execute", "input": input_, "result": failed(code, message, path), "expect": SAME}]}


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
    return out


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
