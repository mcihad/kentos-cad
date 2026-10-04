#!/usr/bin/env python3
"""Independent reference of the project's coordinate systems (docs/adr/0168 §1–§3).

Writes fixtures/geodesy/v1/custom.json from PROJ (pyproj) and the ADR's rules, no KentOS code: points between systems
the project defines and the registry's, through PROJ pipelines written here from the ADR's path rules, each step
explicit (PROJ never picks): a transverse Mercator grid of any origin, scale and false origin on a registry datum or on
a datum of the project's (its ellipsoid and its seven parameters to WGS 84 in either rotation convention); a local
system bound to a projected base by a similarity or an affine plane transform (PROJ's `affine` step); the project's own
seven parameters for a pair of the registry's datums.

A datum of the project's reaches the others through WGS 84 (`+towgs84`'s hub); EPSG's operations elsewhere (ADR 0167
§3). Consecutive Helmert steps stay on Earth-centred coordinates; before the null TUREF–WGS 84 step and at the end
they come back to latitude and longitude on the ellipsoid of the datum the last step landed on. A datum of the
project's with no way to WGS 84 leaves the point without a value in another datum ("noLink").

The core (crates/shared/geometry-core/tests/all/crs_custom.rs) and the web through its WASM must give the same: grid
points within 1e-6 m, latitudes and longitudes within 1e-11 degrees; the accuracy (the steps' sum to the millimetre,
none when one is unknown), what the values rest on and whether an EPSG operation of ED50 was used, exactly.
"""

import argparse
import json
import math
import sys
from decimal import Decimal
from pathlib import Path

import pyproj
from pyproj import CRS, Transformer

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "geodesy" / "v1" / "custom.json"

# ── The registry's datums and EPSG's operations (ADR 0167 §3) ─────────────────────────────────────────────────────────

ELLIPSOIDS = {"TUREF": (6378137.0, 298.257222101), "ED50": (6378388.0, 297.0), "WGS84": (6378137.0, 298.257223563),
              # The hub of ED50 to TUREF's EPSG path, GRS80.
              "ETRS89": (6378137.0, 298.257222101)}

# Position vector parameters: translations (m), rotations (″), scale difference (ppm).
EPSG_1784 = {"translation": [-84.1, -101.8, -129.7], "rotation": [0.0, 0.0, 0.468], "scale": 1.05,
             "convention": "positionVector"}
EPSG_5260 = {"translation": [0.023, 0.036, -0.068], "rotation": [0.00176, 0.00912, -0.01136], "scale": 0.00439,
             "convention": "positionVector"}


def ellipsoid_of(datum):
    if isinstance(datum, str):
        return ELLIPSOIDS[datum]
    e = datum["ellipsoid"]
    return e["semiMajor"], e["inverseFlattening"]


def ell(datum):
    a, rf = ellipsoid_of(datum)
    return f"+a={a!r} +rf={rf!r}"


def helmert_step(h, inverse=False):
    t, r = h["translation"], h["rotation"]
    convention = "position_vector" if h["convention"] == "positionVector" else "coordinate_frame"
    return (f"+step {'+inv ' if inverse else ''}+proj=helmert +x={t[0]!r} +y={t[1]!r} +z={t[2]!r} "
            f"+rx={r[0]!r} +ry={r[1]!r} +rz={r[2]!r} +s={h['scale']!r} +convention={convention}")


def same_datum(a, b):
    """One datum: the registry's by name; the project's by their ellipsoid and their way to WGS 84 (in one
    convention), or, with no way to WGS 84, by their names too."""
    if isinstance(a, str) or isinstance(b, str):
        return a == b
    if ellipsoid_of(a) != ellipsoid_of(b):
        return False
    ha, hb = a.get("toWgs84"), b.get("toWgs84")
    if ha is None or hb is None:
        return ha is None and hb is None and a["name"] == b["name"]
    return pv(ha) == pv(hb)


def pv(h):
    """The parameters in the position vector convention (coordinate frame rotations turned)."""
    sign = 1.0 if h["convention"] == "positionVector" else -1.0
    return (tuple(h["translation"]), tuple(sign * r for r in h["rotation"]), h["scale"])


class NoLink(Exception):
    """A datum of the project's with no way to WGS 84 (§2)."""


def choice_for(choices, a, b):
    for c in choices:
        if (c["from"], c["to"]) == (a, b):
            return c, False
        if (c["from"], c["to"]) == (b, a):
            return c, True
    return None, False


def registry_path(a, b, choices):
    """Steps between two registry datums: the project's choice for the pair, else EPSG's (ADR 0167 §3). A step is
    ("helmert", parameters, reversed, the datum it lands on) or ("null", the datum it lands on): TUREF to WGS 84 (1)
    keeps latitude and longitude. Returns (steps, accuracy or None, via, unofficial)."""
    c, reverse = choice_for(choices, a, b)
    if c is not None:
        return [("helmert", c["helmert"], reverse, b)], c["helmert"].get("accuracy"), c["name"], False
    pair = frozenset([a, b])
    if pair == frozenset(["TUREF", "WGS84"]):
        return [("null", b)], 1.0, "EPSG:5261", False
    if pair == frozenset(["ED50", "WGS84"]):
        return [("helmert", EPSG_1784, a == "WGS84", b)], 2.0, "EPSG:1784", True
    # ED50 to ETRS89 (9), then TUREF to ETRS89 (1) reversed; or the way back.
    if a == "ED50":
        return ([("helmert", EPSG_1784, False, "ETRS89"), ("helmert", EPSG_5260, True, b)], 2.1,
                "EPSG:1783 + EPSG:5260", True)
    return ([("helmert", EPSG_5260, False, "ETRS89"), ("helmert", EPSG_1784, True, b)], 2.1,
            "EPSG:1783 + EPSG:5260", True)


def millimetre(x):
    """The summed accuracy to the millimetre, halves up (an estimate)."""
    return None if x is None else math.floor(x * 1000 + 0.5) / 1000


def datum_path(a, b, choices):
    """The datum steps from `a` to `b` (§2–§3): none within a datum; the registry's ways and the project's choices
    between registry datums; through WGS 84 for a datum of the project's."""
    if same_datum(a, b):
        return [], 0.0, [], False
    if isinstance(a, str) and isinstance(b, str):
        steps, acc, via, unofficial = registry_path(a, b, choices)
        return steps, millimetre(acc), [via], unofficial
    steps, accs, vias, unofficial = [], [], [], False
    # From a datum of the project's to WGS 84.
    if not isinstance(a, str):
        if a.get("toWgs84") is None:
            raise NoLink()
        steps.append(("helmert", a["toWgs84"], False, "WGS84"))
        accs.append(a["toWgs84"].get("accuracy"))
        vias.append(a["name"])
        hub = "WGS84"
    else:
        hub = a
    if not isinstance(b, str):
        if b.get("toWgs84") is None:
            raise NoLink()
        if hub != "WGS84":
            s, acc, via, u = registry_path(hub, "WGS84", choices)
            steps += s
            accs.append(acc)
            vias.append(via)
            unofficial |= u
        steps.append(("helmert", b["toWgs84"], True, b))
        accs.append(b["toWgs84"].get("accuracy"))
        vias.append(b["name"])
    elif hub != b:
        s, acc, via, u = registry_path(hub, b, choices)
        steps += s
        accs.append(acc)
        vias.append(via)
        unofficial |= u
    total = None if any(x is None for x in accs) else sum(accs)
    return steps, millimetre(total), vias, unofficial


# ── Pipelines ──────────────────────────────────────────────────────────────────────────────────────────────────────


def plane_coefficients(plane):
    """The plane transform this system → its base: base x = a·x + b·y + c, base y = d·x + e·y + f."""
    if plane["kind"] == "affine":
        return [plane[k] for k in "abcdef"]
    th = math.radians(plane["rotation"])
    s = plane["scale"]
    return [s * math.cos(th), -s * math.sin(th), plane["east"], s * math.sin(th), s * math.cos(th), plane["north"]]


def affine_step(plane, inverse=False):
    a, b, c, d, e, f = plane_coefficients(plane)
    return (f"+step {'+inv ' if inverse else ''}+proj=affine +xoff={c!r} +yoff={f!r} +s11={a!r} +s12={b!r} "
            f"+s21={d!r} +s22={e!r}")


def tm_args(s):
    return (f"+lat_0={s.get('latitudeOfOrigin', 0.0)!r} +lon_0={s['centralMeridian']!r} +k={s['scaleFactor']!r} "
            f"+x_0={s['falseEasting']!r} +y_0={s['falseNorthing']!r} {ell(s['datum'])}")


def datum_of(s):
    if s["kind"] == "local":
        return datum_of(s["base"])
    if s["kind"] == "mercator":
        return "WGS84"
    return s["datum"]


def unproject(s):
    """Steps from the system's point to its datum's longitude and latitude (radians)."""
    if s["kind"] == "local":
        return [affine_step(s["plane"])] + unproject(s["base"])
    if s["kind"] == "tm":
        return [f"+step +inv +proj=tmerc {tm_args(s)}"]
    if s["kind"] == "mercator":
        return ["+step +inv +proj=webmerc +ellps=WGS84"]
    return ["+step +proj=unitconvert +xy_in=deg +xy_out=rad"]


def project(s):
    if s["kind"] == "local":
        return project(s["base"]) + [affine_step(s["plane"], inverse=True)]
    if s["kind"] == "tm":
        return [f"+step +proj=tmerc {tm_args(s)}"]
    if s["kind"] == "mercator":
        return ["+step +proj=webmerc +ellps=WGS84"]
    return ["+step +proj=unitconvert +xy_in=rad +xy_out=deg"]


def pipeline(src, dst, choices):
    """PROJ's pipeline from `src` to `dst`: the datum steps on Earth-centred coordinates on the ellipsoid of the datum
    each starts from, back to latitude and longitude before a null step and at the end."""
    a, b = datum_of(src), datum_of(dst)
    steps, acc, vias, unofficial = datum_path(a, b, choices)
    parts = unproject(src)
    centred, current = False, a
    for step in steps:
        if step[0] == "helmert":
            if not centred:
                parts.append(f"+step +proj=cart {ell(current)}")
                centred = True
            parts.append(helmert_step(step[1], step[2]))
            current = step[3]
        else:
            if centred:
                parts.append(f"+step +inv +proj=cart {ell(current)}")
                centred = False
            current = step[1]
    if centred:
        parts.append(f"+step +inv +proj=cart {ell(current)}")
    parts += project(dst)
    return "+proj=pipeline " + " ".join(parts), acc, " + ".join(vias), unofficial


# ── Cases ──────────────────────────────────────────────────────────────────────────────────────────────────────────

TUREF_TM30 = {"kind": "tm", "datum": "TUREF", "centralMeridian": 30.0, "scaleFactor": 1.0, "falseEasting": 500000.0,
              "falseNorthing": 0.0}
TUREF_TM33 = dict(TUREF_TM30, centralMeridian=33.0)
ED50_TM30 = dict(TUREF_TM30, datum="ED50")
ED50_TM33 = dict(TUREF_TM33, datum="ED50")
TUREF_GEO = {"kind": "geographic", "datum": "TUREF"}
WGS84_GEO = {"kind": "geographic", "datum": "WGS84"}
PSEUDO = {"kind": "mercator"}

BESSEL = {"name": "Bessel datumu", "ellipsoid": {"name": "Bessel 1841", "semiMajor": 6377397.155,
                                                  "inverseFlattening": 299.1528128},
          "toWgs84": {"translation": [598.1, 73.7, 418.2], "rotation": [0.202, 0.045, -2.455], "scale": 6.7,
                      "convention": "positionVector", "accuracy": 1.5}}
# The same datum written in the coordinate frame convention: the rotations turned.
BESSEL_CF = dict(BESSEL, toWgs84=dict(BESSEL["toWgs84"], rotation=[-0.202, -0.045, 2.455],
                                      convention="coordinateFrame"))
KRASOVSKI = {"name": "Krasovski datumu", "ellipsoid": {"name": "Krasovski 1940", "semiMajor": 6378245.0,
                                                       "inverseFlattening": 298.3},
             "toWgs84": {"translation": [24.0, -123.0, -94.0], "rotation": [0.02, -0.25, -0.13], "scale": 1.1,
                         "convention": "coordinateFrame"}}
LONE = {"name": "Bağsız datum", "ellipsoid": {"name": "Clarke 1880 (RGS)", "semiMajor": 6378249.145,
                                              "inverseFlattening": 293.465}}

CITY = {"kind": "tm", "datum": "TUREF", "latitudeOfOrigin": 36.0, "centralMeridian": 34.5, "scaleFactor": 0.9999,
        "falseEasting": 200000.0, "falseNorthing": 100000.0}
BESSEL_TM = {"kind": "tm", "datum": BESSEL, "latitudeOfOrigin": 0.0, "centralMeridian": 33.0, "scaleFactor": 1.0,
             "falseEasting": 500000.0, "falseNorthing": 0.0}
BESSEL_TM_CF = dict(BESSEL_TM, datum=BESSEL_CF)
KRASOVSKI_GK = {"kind": "tm", "datum": KRASOVSKI, "centralMeridian": 33.0, "scaleFactor": 1.0,
                "falseEasting": 6500000.0, "falseNorthing": 0.0}
LONE_TM = {"kind": "tm", "datum": LONE, "centralMeridian": 33.0, "scaleFactor": 1.0, "falseEasting": 500000.0,
           "falseNorthing": 0.0}
LONE_GEO = {"kind": "geographic", "datum": LONE}
BESSEL_GEO = {"kind": "geographic", "datum": BESSEL}

SITE = {"kind": "local", "base": TUREF_TM33, "plane": {"kind": "similarity", "east": 492345.678, "north": 4422345.678,
                                                       "rotation": 12.5, "scale": 1.000012}}
SITE_AFFINE = {"kind": "local", "base": ED50_TM33,
               "plane": {"kind": "affine", "a": 1.00002, "b": -0.0003, "c": 492000.0, "d": 0.00025, "e": 0.99998,
                         "f": 4422500.0}}
SITE_ON_CITY = {"kind": "local", "base": CITY, "plane": {"kind": "similarity", "east": 1000.0, "north": 2000.0,
                                                         "rotation": -3.25, "scale": 1.0}}

REGION = {"from": "ED50", "to": "TUREF", "name": "ED50 → TUREF: Bölge 7",
          "helmert": {"translation": [-158.785, -109.965, -50.768], "rotation": [1.4275, -3.0873, 0.5505],
                      "scale": -5.1814, "convention": "coordinateFrame", "accuracy": 0.3}}
REGION_UNKNOWN = dict(REGION, helmert={k: v for k, v in REGION["helmert"].items() if k != "accuracy"})
WGS84_ED50 = {"from": "WGS84", "to": "ED50", "name": "WGS 84 → ED50: saha",
              "helmert": {"translation": [87.0, 98.0, 121.0], "rotation": [0.0, 0.0, 0.0], "scale": 0.0,
                          "convention": "positionVector", "accuracy": 1.0}}

# Points of the source system (x east or longitude, y north or latitude).
CITY_POINTS = [[201234.567, 162345.678], [185000.0, 95000.0]]
TM33_POINTS = [[512345.678, 4423456.789], [487000.0, 4300000.0]]
TM30_POINTS = [[412345.678, 4512345.678], [530000.0, 4400000.0]]
SITE_POINTS = [[0.0, 0.0], [1234.567, -876.543]]
GEO_POINTS = [[33.4567891, 39.8765432], [32.0, 37.5]]

TRANSFORMS = [
    # name, from, to, choices, points
    ("başlangıcı ve ölçeği farklı TM'den TUREF TM33'e", CITY, TUREF_TM33, [], CITY_POINTS),
    ("başlangıcı ve ölçeği farklı TM'den WGS 84'e", CITY, WGS84_GEO, [], CITY_POINTS),
    ("TUREF coğrafiden başlangıcı farklı TM'ye", TUREF_GEO, CITY, [], GEO_POINTS),
    ("Bessel datumlu TM'den WGS 84'e", BESSEL_TM, WGS84_GEO, [], TM33_POINTS),
    ("Bessel datumlu TM'den TUREF TM33'e", BESSEL_TM, TUREF_TM33, [], TM33_POINTS),
    ("Bessel datumlu TM'den ED50 TM33'e", BESSEL_TM, ED50_TM33, [], TM33_POINTS),
    ("aynısı, koordinat çerçevesi kuralıyla", BESSEL_TM_CF, ED50_TM33, [], TM33_POINTS),
    ("ED50 TM33'ten Bessel datumlu TM'ye", ED50_TM33, BESSEL_TM, [], TM33_POINTS),
    ("Krasovski Gauss-Krüger'den Bessel coğrafiye", KRASOVSKI_GK,
     BESSEL_GEO, [], [[6512345.678, 4423456.789], [6487000.0, 4300000.0]]),
    ("Bessel coğrafiden Pseudo-Mercator'a", BESSEL_GEO, PSEUDO, [], GEO_POINTS),
    ("Bessel datumlu TM'den ED50'ye, projenin WGS 84 → ED50 seçimiyle", BESSEL_TM, ED50_TM33, [WGS84_ED50],
     TM33_POINTS),
    ("ED50 TM30'dan TUREF TM30'a, projenin bölge parametreleriyle", ED50_TM30, TUREF_TM30, [REGION], TM30_POINTS),
    ("TUREF TM30'dan ED50 TM30'a, aynı seçimin tersi", TUREF_TM30, ED50_TM30, [REGION], TM30_POINTS),
    ("ED50 TM30'dan WGS 84'e: seçim yalnız kendi çiftinde", ED50_TM30, WGS84_GEO, [REGION], TM30_POINTS),
    ("doğruluğu yazılmamış bölge parametreleri", ED50_TM30, TUREF_TM30, [REGION_UNKNOWN], TM30_POINTS),
    ("şantiye sisteminden tabanına (benzerlik)", SITE, TUREF_TM33, [], SITE_POINTS),
    ("şantiye sisteminden WGS 84'e", SITE, WGS84_GEO, [], SITE_POINTS),
    ("TUREF TM33'ten şantiye sistemine", TUREF_TM33, SITE, [], TM33_POINTS),
    ("şantiye sisteminden ED50'ye bağlı afin yerel sisteme", SITE, SITE_AFFINE, [], SITE_POINTS),
    ("afin yerel sistemden ED50 TM33'e", SITE_AFFINE, ED50_TM33, [], SITE_POINTS),
    ("özel TM'ye bağlı yerel sistemden TUREF TM33'e", SITE_ON_CITY, TUREF_TM33, [], SITE_POINTS),
    ("bağsız datumdan kendi coğrafisine", LONE_TM, LONE_GEO, [], TM33_POINTS),
    ("bağsız datumdan TUREF'e", LONE_TM, TUREF_TM33, [], TM33_POINTS),
    ("TUREF'ten bağsız datuma", TUREF_TM33, LONE_TM, [], TM33_POINTS),
]


def build_transforms():
    out = []
    for name, src, dst, choices, points in TRANSFORMS:
        case = {"name": name, "from": src, "to": dst, "choices": choices, "points": points}
        try:
            text, acc, via, unofficial = pipeline(src, dst, choices)
        except NoLink:
            case["expect"] = [{"error": "noLink"} for _ in points]
            out.append(case)
            continue
        t = Transformer.from_pipeline(text)
        expect = []
        for p in points:
            x, y = t.transform(p[0], p[1])
            if not (math.isfinite(x) and math.isfinite(y)):
                expect.append({"error": "outside"})
                continue
            expect.append({"point": [x, y], "accuracy": acc, "via": via, "unofficial": unofficial})
        case["expect"] = expect
        out.append(case)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what PROJ and the rules give")
    args = ap.parse_args()
    data = {"format": "kentos.crs-custom", "version": 1, "proj": pyproj.proj_version_str,
            "transforms": build_transforms()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} PROJ'dan ve kurallardan çıkan değil; yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} PROJ'dan ve kurallardan çıkanla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(data['transforms'])} dönüşüm.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
