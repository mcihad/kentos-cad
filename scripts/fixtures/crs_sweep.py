#!/usr/bin/env python3
"""A randomized difference test of the coordinate transforms against PROJ (docs/adr/0167, 0168).

The shared cases (crs_transform_cases.py, crs_custom_cases.py and the others) pin a few hundred chosen points; this
sweep throws many random ones at the core and at PROJ (pyproj) and says the largest differences:

1. Every ordered pair of the registry's systems (the TM3 and UTM grids, the geographic systems, Pseudo-Mercator):
   points drawn over Türkiye and past its zones' edges, put into the source system by PROJ and moved to the target by
   PROJ's named EPSG way (crs_transform_cases.transformer) and by the core.
2. Random definitions of the project's: transverse Mercators of random origin, scale and false origin on the registry's
   datums and on random datums of the project's (a classic ellipsoid, random seven parameters in either convention),
   local systems over them by a similarity or an affine, random choices of the project's for the registry's pairs.
   PROJ by explicit pipelines written from the ADR's rules (crs_custom_cases.pipeline).
3. PROJ's own meaning of a datum of the project's: the same transverse Mercators written as PROJ strings with
   `+towgs84` (position vector, as PROJ reads it), PROJ choosing its way to the registry's system itself
   (TransformerGroup); where its way is the one the ADR names (EPSG:5261 to TUREF, EPSG:1784 to ED50, none to WGS 84),
   its answers must be the core's. This checks the rule “a datum of the project's goes through WGS 84” itself, which
   part 2 takes for granted. One difference is known and bounded here: on the way to TUREF, PROJ goes back to latitude
   and longitude on GRS80 right after the Helmert step (the null TUREF–WGS 84 step taken as an identity of the
   Earth-centred point), while on the way back from TUREF it takes them on WGS 84 (an identity of latitude and
   longitude); the core takes them on WGS 84 both ways, so that the two ways are each other's inverse (docs/adr/0168
   §9, Doğrulama). The two ellipsoids' flattenings differ by 1.6e-11: at most 0.1 mm, held under 0.2 mm here.

Grid points within 1e-6 m (1e-6 relative in Pseudo-Mercator's millions), latitudes and longitudes within 1e-11
degrees. The core answers through its operations tool (crates/shared/geometry-core/examples/ops.rs, built here in
release). Nothing is written; the exit status says whether every difference is within its tolerance.

    python3 scripts/fixtures/crs_sweep.py [--points 200] [--definitions 40] [--seed 1]
"""

import argparse
import json
import math
import random
import subprocess
import sys
from pathlib import Path

from pyproj import CRS, Transformer
from pyproj.transformer import TransformerGroup

sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_custom_cases as custom  # noqa: E402
import crs_transform_cases as reference  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
TOOL = ROOT / "target" / "release" / "examples" / "ops"

# Where the points fall: Türkiye and past its zones' edges (degrees).
LAT = (35.5, 42.5)
LON = (25.5, 45.0)


class Core:
    """The core's operations tool: one call a line, one answer a line."""

    def __init__(self):
        subprocess.run(["cargo", "build", "-q", "--release", "-p", "kentos-geometry-core", "--example", "ops"],
                       cwd=ROOT, check=True)

    def run(self, calls):
        """The answers to `calls` ([name, args]), in order."""
        text = "".join(f"{name}\t{json.dumps(args, separators=(',', ':'))}\n" for name, args in calls)
        done = subprocess.run([str(TOOL)], input=text, capture_output=True, text=True, check=True)
        return [json.loads(line) for line in done.stdout.splitlines()]


def tolerance(system, known=False):
    """The tolerance; `known` is PROJ's own way to TUREF after a Helmert step (see the docstring): 0.2 mm."""
    if known:
        return 2e-4
    return 1e-11 if system["kind"] == "geographic" else 1e-6


def off(a, b, system):
    """How far apart two answers are, in the tolerance's units: geographic degrees, metres, or Pseudo-Mercator's
    relative metres."""
    d = max(abs(a[0] - b[0]), abs(a[1] - b[1]))
    if system["kind"] == "mercator":
        d /= max(1.0, abs(b[0]), abs(b[1])) / 1e6
    return d


class Report:
    def __init__(self):
        self.worst = {}
        self.failures = []
        self.compared = 0

    def take(self, part, name, system, got, want, known=False):
        self.compared += 1
        if got is None or "error" in got:
            self.failures.append(f"{part}: {name}: çekirdek değer vermedi ({got}) ama PROJ verdi {want}")
            return
        d = off([got["point"]["x"], got["point"]["y"]], want, system)
        if d > self.worst.get(part, (0.0, ""))[0]:
            self.worst[part] = (d, name)
        if d > tolerance(system, known):
            self.failures.append(f"{part}: {name}: fark {d:.3g} > {tolerance(system, known):g}")


def random_places(rng, n):
    return [(rng.uniform(*LAT), rng.uniform(*LON)) for _ in range(n)]


def registry_sweep(core, report, rng, n):
    """Part 1: every ordered pair of the registry's systems."""
    systems = [s for s in json.loads(reference.REGISTRY.read_text(encoding="utf-8"))["systems"] if s["kind"] != "local"]
    for src in systems:
        points = [reference.into(src, lat, lon) for lat, lon in random_places(rng, n)]
        for dst in systems:
            if dst is src:
                continue
            t, _, _ = reference.transformer(src, dst)
            wants = [t.transform(*p) for p in points]
            calls = [["crsTransform", [reference.system(src), reference.system(dst), {"x": p[0], "y": p[1]}]]
                     for p in points]
            for p, got, want in zip(points, core.run(calls), wants):
                if not all(math.isfinite(v) for v in want):
                    continue
                report.take("kayıttaki sistemler", f"{src['name']} → {dst['name']} {p}",
                            reference.system(dst), got, list(want))


ELLIPSOIDS = [("Bessel 1841", 6377397.155, 299.1528128), ("Krasovski 1940", 6378245.0, 298.3),
              ("Clarke 1880 (RGS)", 6378249.145, 293.465), ("International 1924", 6378388.0, 297.0),
              ("GRS 1980", 6378137.0, 298.257222101)]


def random_helmert(rng, with_accuracy=True):
    h = {"translation": [rng.uniform(-700, 700) for _ in range(3)],
         "rotation": [rng.uniform(-10, 10) for _ in range(3)], "scale": rng.uniform(-20, 20),
         "convention": rng.choice(["positionVector", "coordinateFrame"])}
    if with_accuracy:
        h["accuracy"] = round(rng.uniform(0.1, 3.0), 3)
    return h


def random_datum(rng, i):
    if rng.random() < 0.4:
        return rng.choice(["TUREF", "ED50", "WGS84"])
    name, a, rf = rng.choice(ELLIPSOIDS)
    d = {"name": f"Datum {i}", "ellipsoid": {"name": name, "semiMajor": a, "inverseFlattening": rf}}
    if rng.random() < 0.9:
        d["toWgs84"] = random_helmert(rng)
    return d


def random_tm(rng, i):
    s = {"kind": "tm", "datum": random_datum(rng, i)}
    if rng.random() < 0.5:
        s["latitudeOfOrigin"] = round(rng.uniform(0, 40), 6)
    cm = round(rng.uniform(26, 45), 6)
    s.update({"centralMeridian": cm, "scaleFactor": round(rng.uniform(0.9996, 1.0001), 7),
              "falseEasting": round(rng.uniform(0, 1e6), 3), "falseNorthing": round(rng.uniform(-1e5, 1e5), 3)})
    return s


def random_plane(rng):
    if rng.random() < 0.5:
        return {"kind": "similarity", "east": rng.uniform(-5e5, 5e5), "north": rng.uniform(-5e6, 5e6),
                "rotation": rng.uniform(-180, 180), "scale": rng.uniform(0.999, 1.001)}
    th = math.radians(rng.uniform(-180, 180))
    a, b = math.cos(th) * rng.uniform(0.999, 1.001), -math.sin(th) * rng.uniform(0.999, 1.001)
    d, e = -b * rng.uniform(0.999, 1.001), a * rng.uniform(0.999, 1.001)
    return {"kind": "affine", "a": a, "b": b, "c": rng.uniform(-5e5, 5e5), "d": d, "e": e,
            "f": rng.uniform(-5e6, 5e6)}


def random_system(rng, i):
    tm = random_tm(rng, i)
    if rng.random() < 0.3:
        return {"kind": "local", "base": tm, "plane": random_plane(rng)}
    return tm


def random_choices(rng):
    out = []
    for a, b in [("ED50", "TUREF"), ("ED50", "WGS84"), ("TUREF", "WGS84")]:
        if rng.random() < 0.4:
            frm, to = (a, b) if rng.random() < 0.5 else (b, a)
            out.append({"from": frm, "to": to, "name": f"{frm} → {to}: deneme", "helmert": random_helmert(rng)})
    return out


TARGETS = [custom.TUREF_TM30, custom.TUREF_TM33, custom.ED50_TM33, custom.WGS84_GEO, custom.TUREF_GEO, custom.PSEUDO]


def custom_sweep(core, report, rng, n, definitions):
    """Part 2: random definitions against PROJ's explicit pipelines."""
    systems = [random_system(rng, i) for i in range(definitions)]
    for i, src in enumerate(systems):
        choices = random_choices(rng)
        try:
            text, _, _, _ = custom.pipeline(custom.TUREF_GEO, src, [])
        except custom.NoLink:
            continue
        place = Transformer.from_pipeline(text)
        points = [place.transform(lon, lat) for lat, lon in random_places(rng, n)]
        points = [p for p in points if all(math.isfinite(v) for v in p)]
        for dst in TARGETS + [systems[(i + 1) % len(systems)]]:
            try:
                text, _, _, _ = custom.pipeline(src, dst, choices)
            except custom.NoLink:
                calls = [["crsTransformIn", [src, dst, {"x": p[0], "y": p[1]}, choices]] for p in points[:5]]
                for got in core.run(calls):
                    report.compared += 1
                    if got.get("error") != "noLink":
                        report.failures.append(f"tanımlar: {i}: bağsız datumda çekirdek değer verdi: {got}")
                continue
            t = Transformer.from_pipeline(text)
            wants = [t.transform(*p) for p in points]
            calls = [["crsTransformIn", [src, dst, {"x": p[0], "y": p[1]}, choices]] for p in points]
            for p, got, want in zip(points, core.run(calls), wants):
                if not all(math.isfinite(v) for v in want):
                    continue
                report.take("projenin tanımları", f"tanım {i} → {dst['kind']} {p}", dst, got, list(want))


def towgs84(h):
    """PROJ's `+towgs84`: the position vector convention."""
    sign = 1.0 if h["convention"] == "positionVector" else -1.0
    r = [sign * x for x in h["rotation"]]
    return ",".join(repr(v) for v in [*h["translation"], *r, h["scale"]])


# PROJ's way from WGS 84 to each target, by the name the ADR takes (none to WGS 84 itself).
WAYS = {5254: "TUREF to WGS 84 (1)", 2320: "ED50 to WGS 84 (30)", 4326: None}


def semantics_sweep(core, report, rng, n, definitions):
    """Part 3: PROJ's own `+towgs84` against the core."""
    found = 0
    for i in range(definitions):
        tm = random_tm(rng, 1000 + i)
        d = tm["datum"]
        if isinstance(d, str) or "toWgs84" not in d:
            continue
        e = d["ellipsoid"]
        proj = (f"+proj=tmerc +lat_0={tm.get('latitudeOfOrigin', 0.0)!r} +lon_0={tm['centralMeridian']!r} "
                f"+k={tm['scaleFactor']!r} +x_0={tm['falseEasting']!r} +y_0={tm['falseNorthing']!r} "
                f"+a={e['semiMajor']!r} +rf={e['inverseFlattening']!r} +towgs84={towgs84(d['toWgs84'])} "
                f"+units=m +no_defs")
        place = Transformer.from_pipeline(custom.pipeline(custom.TUREF_GEO, tm, [])[0])
        points = [place.transform(lon, lat) for lat, lon in random_places(rng, n)]
        for srid, way in WAYS.items():
            entry = next(s for s in json.loads(reference.REGISTRY.read_text(encoding="utf-8"))["systems"]
                         if s["srid"] == srid)
            group = TransformerGroup(CRS.from_proj4(proj), CRS.from_epsg(srid), always_xy=True)
            chosen = [t for t in group.transformers if (way is None or way in t.description)]
            if not chosen:
                report.failures.append(f"PROJ'un anlamı: tanım {i} → EPSG:{srid}: PROJ'un yolunda “{way}” yok")
                continue
            found += 1
            t = chosen[0]
            dst = reference.system(entry)
            calls = [["crsTransformIn", [tm, dst, {"x": p[0], "y": p[1]}, []]] for p in points]
            # The known difference: PROJ's way to TUREF after the Helmert step (see the docstring).
            known = srid == 5254
            part = "PROJ'un +towgs84 anlamı, TUREF'e (bilinen 0,1 mm)" if known else "PROJ'un +towgs84 anlamı"
            for p, got in zip(points, core.run(calls)):
                want = t.transform(*p)
                report.take(part, f"tanım {i} → EPSG:{srid} {p}", dst, got, list(want), known)
    return found


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--points", type=int, default=200)
    ap.add_argument("--definitions", type=int, default=40)
    ap.add_argument("--seed", type=int, default=1)
    args = ap.parse_args()
    rng = random.Random(args.seed)
    core = Core()
    report = Report()
    registry_sweep(core, report, rng, args.points)
    custom_sweep(core, report, rng, args.points // 2, args.definitions)
    found = semantics_sweep(core, report, rng, args.points // 2, args.definitions)
    for part, (d, name) in report.worst.items():
        print(f"{part}: en büyük fark {d:.3g} ({name})")
    print(f"{report.compared} karşılaştırma; PROJ'un kendi yoluyla {found} tanım-hedef çifti.")
    if report.failures:
        print(f"{len(report.failures)} fark toleransın dışında:", file=sys.stderr)
        for f in report.failures[:30]:
            print(f"  {f}", file=sys.stderr)
        return 1
    print("Bütün farklar toleransın içinde.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
