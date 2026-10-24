#!/usr/bin/env python3
"""Independent reference of `cad.crs.transform` (docs/adr/0519 §2–§4, plan/gis.md CBS-08).

    python3 scripts/fixtures/crs_vertical_cases.py           # writes fixtures/commands/v1/cad.crs.transform.json
    python3 scripts/fixtures/crs_vertical_cases.py --check   # compares

Needs pyproj (PROJ). Without it here, the script runs itself again under QGIS's Python on macOS
(/Applications/QGIS-final-*.app) when there is one.

No KentOS code. The points come from PROJ: between 2D systems the EPSG operation the ADRs name
(crs_transform_cases.py's PATHS: TUREF to WGS 84 (1) EPSG:5261, ED50 to WGS 84 (30) EPSG:1784, ED50 to ETRS89 (9)
+ TUREF to ETRS89 (1) EPSG:1783 + EPSG:5260); with ellipsoidal heights the same operation between the systems
promoted to 3D (`CRS.to_3d()`), where the Helmert moves the height; an orthometric height of one vertical system is
the same height (PROJ's compound systems are checked to keep it), of one datum in another unit PROJ's unit
conversion (MSL height 5714 to MSL height (ft) 8050). The operation's record is written from the ADR's rules: its
steps, a PROJ pipeline built from the EPSG parameters PROJ's database holds (`CoordinateOperation.from_epsg`), the
area of use PROJ's database gives, the summed accuracy. Every pipeline is run through `Transformer.from_pipeline`
and must give PROJ's own answer (1e-6 m, 1e-10°): the record is the operation. What KentOS refuses is checked
against what PROJ would do instead: Antalya height to EGM2008 height has only PROJ's “ballpark” vertical
transformation (accuracy unknown) when no grid is given; that is the silent fallback the ADR rules out.

The vertical systems KentOS knows (kentos_contracts::VERTICAL_SYSTEMS) are checked against PROJ's database: name,
datum, unit and area of use. Grid coordinates are compared within 1e-6 m, degrees within 1e-9 (each case's
`within`); the rest exactly.
"""

import json
import math
import os
import sys
from pathlib import Path

try:
    from pyproj import CRS, Transformer
    from pyproj.crs import CoordinateOperation
    from pyproj.transformer import TransformerGroup
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
sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_transform_cases as ct  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.crs.transform.json"
ENGINE = "KentOS geometry-core (kentos.crs/1)"

# kentos_contracts::VERTICAL_SYSTEMS: code → (name, datum, unit, metres of a unit), checked against PROJ below.
VERTICAL = {
    5775: ("Antalya height", "Antalya", "m", 1.0),
    3855: ("EGM2008 height", "EGM2008 geoid", "m", 1.0),
    5773: ("EGM96 height", "EGM96 geoid", "m", 1.0),
    5714: ("MSL height", "Mean Sea Level", "m", 1.0),
    8050: ("MSL height (ft)", "Mean Sea Level", "ft", 0.3048),
    5735: ("Black Sea height", "Black Sea", "m", 1.0),
}
AREAS = {5775: ("Türkiye (Turkey) - onshore.", (25.62, 35.81, 44.83, 42.15)),
         5735: ("Georgia - onshore.", (39.99, 41.04, 46.72, 43.59))}
ELLIPSOIDS = {"TUREF": (6378137.0, 298.257222101), "ED50": (6378388.0, 297.0), "WGS84": (6378137.0, 298.257223563)}
GRS80 = (6378137.0, 298.257222101)
GRID = "a3f5c2e1d4b6978012ab34cd56ef7890a1b2c3d4e5f60718293a4b5c6d7e8f90"
GEOID = {"name": "TG-20", "sha256": "3b0c6e7c1f2a4d5e8b9a0c1d2e3f405162738495a6b7c8d9e0f1a2b3c4d5e6f7"}
PROJECT_HEIGHTS = {"kind": "orthometric", "verticalSrid": 5775, "geoid": GEOID}

SETUP = {
    "format": "kentos.document", "version": 1, "name": "Kotlar ve dönüşümler",
    "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow", "heights": PROJECT_HEIGHTS,
                 "datumTransforms": [{"from": "ED50", "to": "WGS84", "name": "ED50 → WGS 84: ızgara",
                                      "grid": {"id": GRID, "file": "tr_ed50_wgs84.gsb", "size": 4096,
                                               "accuracy": 0.3}}]},
    "origin": {"x": 500000, "y": 4400000},
    "layers": [{"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
    "activeLayer": "0", "entities": [], "styles": {"items": [], "categories": []},
}
SAME = {"ids": [], "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def n(v):
    """A number as Rust's `{}` writes an f64: the shortest text that reads back, whole numbers without a point."""
    v = float(v)
    if v == int(v) and abs(v) < 1e16:
        return str(int(v))
    text = repr(v)
    assert "e" not in text, text
    return text


# ── The registry and PROJ's database ───────────────────────────────────────────────────────────────────────────


REGISTRY = {e["srid"]: e for e in json.loads(ct.REGISTRY.read_text(encoding="utf-8"))["systems"]}


def entry(srid):
    return REGISTRY[srid]


def title(srid):
    e = entry(srid)
    return f"{e['name']} (EPSG:{srid})"


def check_registry():
    for code, (name, datum, unit, metres) in VERTICAL.items():
        crs = CRS.from_epsg(code)
        ax = crs.axis_info[0]
        assert crs.name == name and crs.datum.name == datum, (code, crs.name, crs.datum.name)
        assert ax.unit_conversion_factor == metres and ax.direction == "up", (code, ax)
        area = AREAS.get(code, ("World.", (-180.0, -90.0, 180.0, 90.0)))
        assert crs.area_of_use.name == area[0] and tuple(crs.area_of_use.bounds) == area[1], (code, crs.area_of_use)


# The EPSG operations' parameters, as PROJ's database holds them.
def helmert_of(code):
    op = CoordinateOperation.from_epsg(code)
    p = {q.name: q.value for q in op.params}
    assert op.method_name.startswith("Position Vector transformation"), op.method_name
    return {"x": p["X-axis translation"], "y": p["Y-axis translation"], "z": p["Z-axis translation"],
            "rx": p["X-axis rotation"], "ry": p["Y-axis rotation"], "rz": p["Z-axis rotation"],
            "s": p["Scale difference"], "accuracy": op.accuracy}


TURKIYE = ("Türkiye (Turkey) - onshore and offshore.", (25.62, 34.42, 44.83, 43.45))
for _code in (5261, 1784, 1783, 5260):
    _a = CoordinateOperation.from_epsg(_code).area_of_use
    assert (_a.name, tuple(_a.bounds)) == TURKIYE, (_code, _a)


def datum_steps(a, b):
    """The ADR's way between two of the registry's datums: (kind, name, reverse, accuracy, helmert, lands)."""
    if a == b:
        return []
    if {a, b} == {"TUREF", "WGS84"}:
        return [("null", "EPSG:5261", False, CoordinateOperation.from_epsg(5261).accuracy, None, ELLIPSOIDS[b])]
    if {a, b} == {"ED50", "WGS84"}:
        h = helmert_of(1784)
        return [("helmert", "EPSG:1784", a == "WGS84", h["accuracy"], h, ELLIPSOIDS[b])]
    h9, h1 = helmert_of(1783), helmert_of(5260)
    if a == "ED50":
        return [("helmert", "EPSG:1783", False, h9["accuracy"], h9, GRS80),
                ("helmert", "EPSG:5260", True, h1["accuracy"], h1, ELLIPSOIDS[b])]
    return [("helmert", "EPSG:5260", False, h1["accuracy"], h1, GRS80),
            ("helmert", "EPSG:1783", True, h9["accuracy"], h9, ELLIPSOIDS[b])]


def datum(srid):
    return ct.datum(entry(srid))


def ell(e):
    return f"+a={n(e[0])} +rf={n(e[1])}"


def tm(e):
    return (f"+proj=tmerc +lat_0=0 +lon_0={n(e['centralMeridian'])} +k={n(e['scaleFactor'])} "
            f"+x_0={n(e['falseEasting'])} +y_0={n(e['falseNorthing'])} {ell(ELLIPSOIDS[e['datum']])}")


WEBMERC = "+proj=webmerc +a=6378137 +rf=298.257223563"


def conversion(e, reverse):
    """The pipeline step and the record's step of a system's projection; none for a geographic system."""
    if e["kind"] == "geographic":
        return ("+proj=unitconvert +xy_in=deg +xy_out=rad" if reverse else "+proj=unitconvert +xy_in=rad +xy_out=deg"), None
    if e.get("projection") == "Pseudo-Mercator":
        return ("+inv " if reverse else "") + WEBMERC, step("conversion", "Pseudo-Mercator", reverse, 0.0)
    return ("+inv " if reverse else "") + tm(e), step("conversion", f"TM {n(e['centralMeridian'])}°", reverse, 0.0)


def step(kind, name, reverse, accuracy):
    return {"kind": kind, "name": name, "reverse": reverse, "accuracy": accuracy}


def operation(src, dst, keep_height, unit=None):
    """The ADR's record of the way from `src` to `dst` (EPSG codes): id, steps, pipeline, accuracy, area, unofficial."""
    a, b = datum(src), datum(dst)
    shifts = datum_steps(a, b)
    pipe, steps = [], []
    p, s = conversion(entry(src), True)
    pipe.append(p)
    if s:
        steps.append(s)
    if shifts and keep_height:
        pipe.append("+proj=push +v_3")
        pipe.append("+proj=set +v_3=0")
    now, geographic = ELLIPSOIDS[a], True
    for kind, name, reverse, accuracy, h, lands in shifts:
        if kind == "helmert":
            if geographic:
                pipe.append(f"+proj=cart {ell(now)}")
                geographic = False
            pipe.append(f"{'+inv ' if reverse else ''}+proj=helmert +x={n(h['x'])} +y={n(h['y'])} +z={n(h['z'])} "
                        f"+rx={n(h['rx'])} +ry={n(h['ry'])} +rz={n(h['rz'])} +s={n(h['s'])} "
                        "+convention=position_vector")
        elif kind == "null" and not geographic:
            pipe.append(f"+inv +proj=cart {ell(now)}")
            geographic = True
        steps.append(step(kind, name, reverse, accuracy))
        now = lands
    if not geographic:
        pipe.append(f"+inv +proj=cart {ell(now)}")
    if shifts and keep_height:
        pipe.append("+proj=pop +v_3")
    p, s = conversion(entry(dst), False)
    pipe.append(p)
    if s:
        steps.append(s)
    if unit:
        pipe.append(f"+proj=unitconvert +z_in={unit[0]} +z_out={unit[1]}")
        steps.append(step("unit", f"EPSG:{unit[2]} → EPSG:{unit[3]}", False, 0.0))
    total = sum(x[3] for x in shifts)
    accuracy = math.floor(total * 1000 + 0.5) / 1000 if shifts else 0.0
    via = ct.PATHS[frozenset([a, b])] if a != b else None
    if via:
        assert accuracy == via[1], (accuracy, via)
    rec = {"id": via[2] if via else "", "steps": steps, "pipeline": "+proj=pipeline" + "".join(f" +step {x}" for x in pipe),
           "engine": ENGINE, "grids": [], "accuracy": accuracy, "epochApplied": False,
           "unofficial": "ED50" in (a, b) and a != b}
    if shifts:
        rec["area"] = {"name": TURKIYE[0], "west": TURKIYE[1][0], "south": TURKIYE[1][1], "east": TURKIYE[1][2],
                       "north": TURKIYE[1][3]}
    return rec


# ── PROJ's answers ────────────────────────────────────────────────────────────────────────────────────────────────


def proj_2d(src, dst):
    t, _, _ = ct.transformer(entry(src), entry(dst))
    return lambda x, y: t.transform(x, y)


def proj_3d(src, dst):
    """The ADR's operation between the systems promoted to 3D: the height rides the Helmert."""
    a, b = datum(src), datum(dst)
    s3, d3 = CRS.from_epsg(src).to_3d(), CRS.from_epsg(dst).to_3d()
    if a == b:
        t = Transformer.from_crs(s3, d3, always_xy=True)
    else:
        name = ct.PATHS[frozenset([a, b])][0]
        t = next(t for t in TransformerGroup(s3, d3, always_xy=True).transformers if name in t.description)
    return lambda x, y, z: t.transform(x, y, z)


def place(srid, lat, lon):
    return ct.into(entry(srid), lat, lon)


BURSA, ESKISEHIR = (40.1826, 29.0665), (39.7767, 30.5206)


def pt(x, y, z=None):
    out = {"x": x, "y": y}
    if z is not None:
        out["z"] = z
    return out


# ── Titles and messages (the command's words) ──────────────────────────────────────────────────────────────────


def heights_title(h):
    if h is None:
        return None
    if h["kind"] == "ellipsoidal":
        return "elipsoit yüksekliği"
    t = "ortometrik"
    v = h.get("verticalSrid")
    t += f", {VERTICAL[v][0]} (EPSG:{v})" if v in VERTICAL else ", düşey sistemi belirtilmemiş"
    if h.get("geoid"):
        t += f", geoit {h['geoid']['name']}"
    return t


def side_title(srid, heights, epoch):
    t = title(srid)
    if heights:
        t += " + " + heights_title(heights)
    if epoch is not None:
        t += f" @ {n(epoch)}"
    return t


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def warning(code, message, path):
    return {"code": code, "message": message, "path": path}


UNOFFICIAL = ("EPSG:1783 + EPSG:5260 EPSG'nin ED50 işlemi: değerleri resmî dönüşümün değil (Tapu ve Kadastro'nun bölge "
              "parametreleri için Proje ayarları › Datum dönüşümleri).")
EPOCH_NOT_APPLIED = warning("epoch_not_applied",
                            "Epok kayıtta; seçilen işlem zamana bağlı değil, koordinatlara uygulanmadı.", "from.epoch")
HEIGHTS_UNCHANGED = warning("heights_unchanged",
                            "Kotların ne olduğu iki tarafta da söylenmedi; kotlar değişmeden kaldı. Elipsoit "
                            "yüksekliğiyse datum değişince değişirdi: yüksekliği söyleyin (heights).", "points")


def rss(a, b):
    if a is None or b is None:
        return None
    return round(math.sqrt(a * a + b * b) * 1000) / 1000


# ── The cases ───────────────────────────────────────────────────────────────────────────────────────────────────


def resolve(side):
    """A side's EPSG code, heights (None: not said) and epoch, the project's taken where the side is the project's."""
    side = side or {}
    own = "srid" not in side
    srid = side.get("srid", 5254)
    heights = side.get("heights", PROJECT_HEIGHTS if own else None)
    if heights and heights["kind"] == "unknown":
        heights = None
    return srid, heights, side.get("epoch")


def transformed(points, frm=None, to=None, uncertainty=None, within=None, op="execute"):
    """A completed case, every value from PROJ and the ADR's rules."""
    src, sh, se = resolve(frm)
    dst, dh, de = resolve(to)
    has_z = any("z" in p for p in points)
    method, unit = "none", None
    if has_z:
        if sh is None and dh is None:
            method = "unchanged"
        elif sh["kind"] == dh["kind"] == "ellipsoidal":
            method = "ellipsoidal"
        elif sh.get("verticalSrid") == dh.get("verticalSrid"):
            method = "same"
        else:
            a, b = VERTICAL[sh["verticalSrid"]], VERTICAL[dh["verticalSrid"]]
            assert a[1] == b[1]
            method, unit = "unit", (a[2], b[2], sh["verticalSrid"], dh["verticalSrid"], a[3] / b[3])
    rec = operation(src, dst, method != "ellipsoidal", unit)
    p2, p3 = proj_2d(src, dst), proj_3d(src, dst)
    pipeline = Transformer.from_pipeline(rec["pipeline"])
    out, unreached = [], []
    geographic_out = entry(dst)["kind"] == "geographic"
    tol = 1e-10 if geographic_out else 1e-6
    for i, q in enumerate(points):
        if entry(src)["kind"] == "geographic" and abs(q["y"]) > 90:
            out.append(None)
            unreached.append({"index": i, "reason": "outside"})
            continue
        z = q.get("z")
        if method == "ellipsoidal" and z is not None:
            x, y, z2 = p3(q["x"], q["y"], z)
        else:
            x, y = p2(q["x"], q["y"])
            z2 = z * unit[4] if (method == "unit" and z is not None) else z
        if method == "unit" and z is not None:
            # PROJ's own unit conversion between the two vertical systems of one datum.
            t = Transformer.from_crs(f"EPSG:{src}+{unit[2]}", f"EPSG:{dst}+{unit[3]}", always_xy=True)
            assert abs(t.transform(q["x"], q["y"], z)[2] - z2) < 1e-9
        if method == "same" and z is not None and datum(src) != datum(dst):
            # PROJ keeps an orthometric height of one vertical system through the horizontal datum's shift.
            vs = sh["verticalSrid"]
            t = Transformer.from_crs(f"EPSG:{src}+{vs}", f"EPSG:{dst}+{vs}", always_xy=True)
            assert abs(t.transform(q["x"], q["y"], z)[2] - z) < 1e-9
        # The record's pipeline gives PROJ's answer.
        px, py, pz = pipeline.transform(q["x"], q["y"], z if z is not None else 0.0)
        assert abs(px - x) < tol and abs(py - y) < tol, (rec["pipeline"], px - x, py - y)
        if z is not None:
            assert abs(pz - z2) < 1e-6, (rec["pipeline"], pz, z2)
        out.append(pt(x, y, z2))
    height_accuracy = {"none": None, "unchanged": None, "same": 0.0, "unit": 0.0,
                       "ellipsoidal": rec["accuracy"]}[method]
    rec.update({"from": side_title(src, sh, se), "to": side_title(dst, dh, de), "heights": method})
    if height_accuracy is not None:
        rec["heightAccuracy"] = height_accuracy
    from_epoch, to_epoch = se, de if de is not None else se
    if from_epoch is not None:
        rec["fromEpoch"] = from_epoch
    if to_epoch is not None:
        rec["toEpoch"] = to_epoch
    warnings = []
    if method == "unchanged":
        warnings.append(HEIGHTS_UNCHANGED)
    if from_epoch is not None or to_epoch is not None:
        warnings.append(EPOCH_NOT_APPLIED)
    if rec["unofficial"]:
        warnings.append(warning("unofficial_operation", UNOFFICIAL, "to"))
    if unreached:
        warnings.append(warning("points_outside",
                                f"{len(unreached)} noktanın hedef sistemde değeri yok (izdüşümün ya da ızgaranın "
                                "dışında); yerleri boş.", "points"))
    source = uncertainty or {}
    report = {"source": source, "topologyTolerance": 0.001, "displayDecimals": 3, "displayStep": 0.001,
              "transformation": rec["accuracy"]}
    for key, value in (("transformationVertical", height_accuracy),
                       ("combined", rss(source.get("horizontal"), rec["accuracy"])),
                       ("combinedVertical", rss(source.get("vertical"), height_accuracy))):
        if value is not None:
            report[key] = value
    inp = {"points": points}
    for k, v in (("from", frm), ("to", to), ("uncertainty", uncertainty)):
        if v is not None:
            inp[k] = v
    s = {"op": op, "input": inp,
         "result": {"status": "completed",
                    "output": {"points": out, "unreached": unreached, "operation": rec, "report": report},
                    "warnings": warnings},
         "expect": SAME, "within": within or tol}
    return s


def refused(points, code, message, path, frm=None, to=None, uncertainty=None, **more):
    inp = {"points": points}
    for k, v in (("from", frm), ("to", to), ("uncertainty", uncertainty)):
        if v is not None:
            inp[k] = v
    s = {"op": "execute", "input": inp, "result": failed(code, message, path), "expect": SAME}
    s.update(more)
    return s


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def ballpark_only(a, b):
    """Whether PROJ's only ways between two compound systems are its ballpark ones (accuracy unknown)."""
    g = TransformerGroup(a, b, always_xy=True)
    return g.transformers and all(t.accuracy == -1 and "ballpark" in t.description for t in g.transformers)


def cases():
    check_registry()
    own = place(5254, *BURSA)
    own2 = place(5254, *ESKISEHIR)
    ed50 = place(2320, *BURSA)
    ell = {"kind": "ellipsoidal"}
    ortho = lambda v: {"kind": "orthometric", "verticalSrid": v}  # noqa: E731
    out = []
    out.append(case("Projenin sisteminden ED50 / TM30'a, kotsuz: iki Helmert, resmî olmayan işlem",
                    transformed([pt(*own), pt(*own2)], to={"srid": 2320})))
    out.append(case("ED50 / TM30'dan TUREF / TM30'a elipsoit yüksekliğiyle: kot Helmert'le değişir",
                    transformed([pt(*ed50, 812.4)], frm={"srid": 2320, "heights": ell},
                                to={"srid": 5254, "heights": ell})))
    out.append(case("Aynısının planı: aynı cevap, çizim değişmez",
                    transformed([pt(*ed50, 812.4)], frm={"srid": 2320, "heights": ell},
                                to={"srid": 5254, "heights": ell}, op="plan")))
    out.append(case("TUREF / TM30'dan WGS 84'e elipsoit yüksekliğiyle: boş dönüşüm, kot aynen kalır",
                    transformed([pt(*own, 1043.25)], frm={"srid": 5254, "heights": ell},
                                to={"srid": 4326, "heights": ell}, within=1e-9)))
    wgs = (BURSA[1], BURSA[0])
    out.append(case("WGS 84'ten UTM 35N'e: yalnız izdüşüm, doğruluk 0, kullanım alanı yok",
                    transformed([pt(*wgs, 95.5)], frm={"srid": 4326, "heights": ell},
                                to={"srid": 32635, "heights": ell})))
    out.append(case("Ortometrik kot datum değişirken kalır: Antalya kotu ED50 / TM30'a (push ve pop)",
                    transformed([pt(*own, 120.75)], to={"srid": 2320, "heights": ortho(5775)})))
    out.append(case("Aynı düşey datumun başka birimi: MSL metre → MSL ayak",
                    transformed([pt(*own, 100.0)], frm={"srid": 5254, "heights": ortho(5714)},
                                to={"srid": 5254, "heights": ortho(8050)})))
    assert ballpark_only("EPSG:5254+5775", "EPSG:5254+3855"), "PROJ has more than a ballpark way"
    out.append(case("Antalya kotundan EGM2008'e: iki düşey datum, PROJ'un ballpark'ı yerine ret",
                    refused([pt(*own, 120.75)], "vertical_unsupported",
                            f"Kotlar “{heights_title(PROJECT_HEIGHTS)}” ile “{heights_title(ortho(3855))}” arasında "
                            "çevrilmeli: iki ayrı düşey datum. Bunun için iki yanın da geoit modeli SHA-256'sıyla "
                            "gerekir (heights.geoid); PROJ'un yaptığı gibi kotları olduğu gibi bırakan “kaba” bir yol "
                            "kullanılmaz.", "to.heights.verticalSrid",
                            to={"srid": 5254, "heights": ortho(3855)})))
    out.append(case("Elipsoit yüksekliğinden projenin ortometrik kotuna: projenin geoidi bu cihazda yok",
                    refused([pt(*own, 1043.25)], "geoid_missing",
                            f"“TG-20” geoit modelinin ızgarası ({GEOID['sha256'][:12]}…) bu cihazın ızgara kitaplığında "
                            "yok. Izgarayı Proje ayarları › Izgaralar'a ekleyin; daha düşük doğruluklu bir yola geçilmez.",
                            "to.heights.geoid", frm={"srid": 5254, "heights": ell})))
    out.append(case("Bir yanın yüksekliği söylenmemiş: kotlar varsayımla çevrilmez",
                    refused([pt(*own, 120.75)], "heights_unknown",
                            "Kotlar verildi; to tarafının yüksekliği bilinmiyor, öbürününki biliniyor. Kotlar bir "
                            "varsayımla çevrilmez: iki tarafın da yüksekliğini (heights) söyleyin ya da kotları vermeyin.",
                            "to.heights", to={"srid": 5256})))
    out.append(case("İki yanın yüksekliği de bilinmiyor: kotlar kalır, uyarıyla",
                    transformed([pt(*own, 120.75)], frm={"srid": 5254, "heights": {"kind": "unknown"}},
                                to={"srid": 5256})))
    out.append(case("Kotsuz noktada yüksekliklerin farkı reddedilmez",
                    transformed([pt(*own)], frm={"srid": 5254, "heights": ell}, to={"srid": 5256})))
    out.append(case("İki ayrı epok: zamana bağlı dönüşüm yok, ret",
                    refused([pt(*own)], "epoch_unsupported",
                            "Koordinatlar 2005 epokundan 2024 epokuna taşınmalı: bunun için hız modeli gerekir. Hız "
                            "ızgarasını (doğu, kuzey, yukarı mm/yıl; GeoTIFF) Proje ayarları › Izgaralar'a ekleyip "
                            "velocity ile verin; koordinatlar epok farkı yok sayılarak çevrilmez.", "to.epoch", frm={"srid": 5254, "epoch": 2005.0},
                            to={"srid": 4326, "epoch": 2024.0})))
    out.append(case("Aynı epok iki yanda: kayıtta, uygulanmadı uyarısıyla",
                    transformed([pt(*own, 1043.25)], frm={"srid": 5254, "heights": ell, "epoch": 2023.5},
                                to={"srid": 4326, "heights": ell, "epoch": 2023.5}, within=1e-9)))
    out.append(case("Yalnız kaynağın epoku: hedefin epoku da odur",
                    transformed([pt(*own)], frm={"srid": 5254, "epoch": 2023.5}, to={"srid": 5256})))
    out.append(case("Kaynağın belirsizliği raporda ayrı; toplamı karelerin kökü",
                    transformed([pt(*ed50, 812.4)], frm={"srid": 2320, "heights": ell},
                                to={"srid": 5254, "heights": ell},
                                uncertainty={"horizontal": 0.05, "vertical": 0.02})))
    out.append(case("Pseudo-Mercator'dan projenin sistemine, kotsuz",
                    transformed([pt(*place(3857, *BURSA))], frm={"srid": 3857})))
    out.append(case("Enlemi 90°'yi aşan nokta: değeri yok, öbürü dönüşür",
                    transformed([pt(29.0, 95.0), pt(*wgs)], frm={"srid": 4326})))
    out.append(case("Projenin ED50 ızgarası bu cihazda yok: daha düşük doğruluklu yola geçilmez",
                    refused([pt(*ed50)], "grid_missing",
                            "Projenin bu iki datum için seçtiği ızgara bu cihazda yok. Izgarayı Proje ayarları › "
                            "Izgaralar'a ekleyin; EPSG'nin daha düşük doğruluklu yoluna kendiliğinden geçilmez.", "to",
                            frm={"srid": 2320}, to={"srid": 4326})))
    out.append(case("Nokta yok",
                    refused([], "nothing_to_transform",
                            "0 nokta verildi; en az 1, en çok 100 000 nokta dönüştürülür.", "points")))
    out.append(case("Sonlu olmayan doğu",
                    refused([pt(*own)], "not_finite",
                            "1. noktanın doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir "
                            "sayıyla verin.", "points[0].x", nonFinite={"points[0].x": "Infinity"})))
    out.append(case("Sonlu olmayan kot",
                    refused([pt(*own), pt(*own2, 10.0)], "not_finite",
                            "2. noktanın kotu sonlu bir sayı değil (NaN ya da sonsuz). Kotu sonlu bir sayıyla verin ya "
                            "da bırakın.", "points[1].z", nonFinite={"points[1].z": "NaN"})))
    out.append(case("Kayıtta olmayan sistem",
                    refused([pt(*own)], "unknown_srid",
                            "EPSG:9999 koordinat sistemleri kaydında yok ya da bir yere bağlı değil. Kayıttaki bir "
                            "sistemin kodunu verin ya da kodu bırakın (projenin sistemi).", "to.srid",
                            to={"srid": 9999})))
    out.append(case("Yerel sistem (0) bir yere bağlı değil",
                    refused([pt(*own)], "unknown_srid",
                            "EPSG:0 koordinat sistemleri kaydında yok ya da bir yere bağlı değil. Kayıttaki bir "
                            "sistemin kodunu verin ya da kodu bırakın (projenin sistemi).", "from.srid",
                            frm={"srid": 0})))
    out.append(case("Elipsoit yüksekliği düşey sistem anmaz",
                    refused([pt(*own)], "invalid_heights",
                            "from yüksekliği: düşey sistem ve geoit modeli yalnız ortometrik yüksekliğin olur; elipsoit "
                            "yüksekliğinin elipsoidi yatay sistemindir.", "from.heights.kind",
                            frm={"srid": 5254, "heights": {"kind": "ellipsoidal", "verticalSrid": 3855}})))
    out.append(case("Bilinmeyen düşey sistem",
                    refused([pt(*own)], "invalid_heights",
                            "to yüksekliği: EPSG:5703 KentOS'un bildiği düşey sistemlerden değil (Antalya, EGM2008, "
                            "EGM96, MSL, Karadeniz).", "to.heights.verticalSrid",
                            to={"srid": 5254, "heights": ortho(5703)})))
    out.append(case("Epok yılların dışında",
                    refused([pt(*own)], "invalid_epoch",
                            "from epoku: koordinat epoku 1900 ile 2100 arasında ondalık bir yıl olmalı (2023.5 gibi).",
                            "from.epoch", frm={"epoch": 1850.0})))
    out.append(case("Eksi belirsizlik",
                    refused([pt(*own)], "invalid_uncertainty",
                            "Kaynağın belirsizliği 0 ya da büyük sonlu bir sayı olmalı (metre).",
                            "uncertainty.horizontal", uncertainty={"horizontal": -0.01})))
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.crs.transform", "commandVersion": 1,
        "title": "Koordinat dönüştür: kotlar, epoklar, işlemin kaydı ve doğruluk raporu",
        "note": "ADR 0519. Noktalar PROJ'dan (pyproj): 2B'de ADR 0167'nin EPSG işlemleri, elipsoit yüksekliğiyle aynı "
                "işlem 3B sistemler arasında; kaydın pipeline'ı EPSG parametreleriyle kurulur ve PROJ'da aynı cevabı "
                "verir. Geoit modeli ve hız ızgarası olmadan geoit, düşey datum ve epok dönüşümü reddedilir (ADR 0631: "
                "cad.crs.transform.geoid.json). Proje: TUREF / TM30, Antalya kotu (TG-20), "
                "ED50–WGS 84 için cihazda olmayan bir ızgara. Bağımsız başvuru: scripts/fixtures/crs_vertical_cases.py.",
        "setup": SETUP, "cases": cases(),
    }


def main():
    built = build()
    text = json.dumps(built, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"koordinat dönüşümünün durumları tutarlı: {len(built['cases'])} durum (PROJ {__import__('pyproj').proj_version_str})")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(built['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
