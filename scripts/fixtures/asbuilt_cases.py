#!/usr/bin/env python3
"""The shared cases of `cad.asbuilt.check` (Uygulama kontrolü; plan/map-cad.md MAP-19, docs/adr/1700), without KentOS
code.

    python3 scripts/fixtures/asbuilt_cases.py           # writes fixtures/commands/v1/cad.asbuilt.check.json
    python3 scripts/fixtures/asbuilt_cases.py --check   # writes nothing; compares

A Harita project at TUREF / TM36 coordinates (from X0, Y0): the design surface “Tasarım”, a plane
G(x, y) = 100 + 0.02 x + 0.01 y over [0, 100]², its corners every 10 m; the design points on `tasarim` (names in Ad,
heights on the plane, P8 without one, P5 and P6 0.6 m apart); the kerb lines on `bordur` (a polyline east along
y = 30, a line north along x = 50); the points measured on `olcum` with their σ in Hassasiyet.

The rules, by hand (Trimble Business Center's Point and String Conformance reports' questions):
- a measured point matches the one design point of its name; two of the name: ambiguous; none and a radius given:
  the design points within it that no measured point claims by name, one a proposal (`place`), two or more
  ambiguous;
- the differences are measured − design: east, north, height, the horizontal distance; the surface difference is the
  measured height − G; the normal offset is to the nearest point of the nearest design line, left of its direction
  positive, for a point no design point matches and (a radius given) within it of a line;
- a limit not given is not checked; σ beyond its limit flags `lowQuality` and leaves the status;
- the status: ambiguous; unmatched when neither a design point nor a line takes it (a surface alone takes every
  point); else outside (a limit broken), within;
- the digest: SHA-256 of `p<TAB>name<TAB>x<TAB>y<TAB>z` (six decimals, `-` for none) for each design point, then
  `l<TAB>x,y;x,y` for each design line, each line ended by a newline;
- the package's own design (docs/adr/2820): its `noktalar.csv` and `cizgiler.csv` are the design compared, read back
  to its digest (else refused); the drawing's layers, given, only say whether the design changed since;
- the package's own surface (docs/adr/3220): a package that carries `yuzey.xml` (a LandXML TIN, its SHA-256 in the
  manifest) has it compared with its design, the surface given or not; the surface given then only says whether the
  drawing's changed since (the manifest's sources' digest not the drawing's: here “0…0”, the drawing's surface moved
  on). The package's surface PG is the plane 5 cm under G over the south half, [0, 100] × [0, 50], written here as
  four corners and two faces; a point north of it is off the surface. A file not its SHA-256's, or not in the
  package, is refused.
"""

import copy
import hashlib
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.asbuilt.check.json"
# The field packages of docs/adr/2420, written here with Python's zipfile (no KentOS code).
PACKAGES = ROOT / "fixtures" / "asbuilt" / "v1" / "files"
X0, Y0 = 487000.0, 4420000.0
W = 1e-6
FLAGS = ["proposed", "outsideHorizontal", "outsideVertical", "outsideSurface", "outsideLine", "lowQuality",
         "noHeight", "offSurface"]


def uid(n):
    return f"0192f700-1700-7000-8000-{n:012x}"


def G(x, y):
    if not (0.0 <= x <= 100.0 and 0.0 <= y <= 100.0):
        return None
    return 100.0 + 0.02 * x + 0.01 * y


def PG(x, y):
    """The package's surface (docs/adr/3220): the design surface as the crew had it, 5 cm under G, its south half."""
    if not (0.0 <= x <= 100.0 and 0.0 <= y <= 50.0):
        return None
    return 100.0 + 0.02 * x + 0.01 * y - 0.05


PG_CORNERS = [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)]
PG_FACES = [(1, 2, 3), (1, 3, 4)]


def tin_xml(corners, faces, z):
    """A LandXML 1.2 TIN of `corners` (x, y from X0, Y0; heights by `z`), its faces 1-based, written here (no KentOS
    code): each point “northing easting height”, each number Python's shortest text that reads back the same."""
    pts = "".join(f'          <P id="{k}">{Y0 + y!r} {X0 + x!r} {z(x, y)!r}</P>\n' for k, (x, y) in enumerate(corners, 1))
    fs = "".join(f"          <F>{a} {b} {c}</F>\n" for a, b, c in faces)
    return ('<?xml version="1.0" encoding="UTF-8"?>\n'
            '<LandXML xmlns="http://www.landxml.org/schema/LandXML-1.2" version="1.2" date="2026-11-08" '
            'time="09:00:00">\n'
            '  <Units>\n    <Metric areaUnit="squareMeter" linearUnit="meter" volumeUnit="cubicMeter" '
            'angularUnit="decimal degrees" directionUnit="decimal degrees"/>\n  </Units>\n'
            '  <Surfaces>\n    <Surface name="Tasarım">\n      <Definition surfType="TIN">\n        <Pnts>\n'
            f'{pts}        </Pnts>\n        <Faces>\n{fs}        </Faces>\n      </Definition>\n    </Surface>\n'
            '  </Surfaces>\n</LandXML>\n')


DESIGN = [("P1", 10.0, 10.0, 100.3), ("P2", 20.0, 10.0, 100.5), ("P3", 30.0, 10.0, 100.7), ("P4", 40.0, 10.0, 100.9),
          ("P5", 60.0, 60.0, 101.8), ("P6", 60.6, 60.0, 101.812), ("P7", 80.0, 80.0, 102.4), ("P8", 90.0, 10.0, None)]
# (kind, points)
LINES = [("polyline", [(0.0, 30.0), (100.0, 30.0)]), ("line", [(50.0, 40.0), (50.0, 90.0)])]
# (name, x, y, z, σ)
MEASURED = [("P1", 10.01, 10.0, 100.31, "0.01"), ("P2", 20.0, 10.05, 100.5, "0.01"), ("P3", 30.0, 10.0, 100.74, None),
            ("p4x", 40.1, 10.05, 100.9, None), ("X9", 60.3, 60.1, 101.8, None), ("R1", 70.0, 20.0, 101.6, None),
            ("S1", 10.0, 20.0, None, None), ("P8", 90.0, 10.0, 101.9, "0.08"), ("B1", 25.0, 29.97, 100.8, None),
            ("B2", 50.01, 60.0, 101.6, "0,02")]


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def setup(work="map"):
    layers = [layer("0", "0"), layer("zemin", "Tasarım yüzeyi"), layer("tasarim", "Tasarım noktaları"),
              layer("bordur", "Bordür"), layer("olcum", "Ölçülen"), layer("bos", "Boş")]
    ents = []

    def add(e):
        e["id"] = len(ents) + 1
        ents.append(e)

    for j in range(11):
        for i in range(11):
            x, y = 10.0 * i, 10.0 * j
            add({"kind": "point", "layerId": "zemin", "attrs": {}, "p": P(x, y), "z": G(x, y)})
    for name, x, y, z in DESIGN:
        e = {"kind": "point", "layerId": "tasarim", "attrs": {"Ad": name}, "p": P(x, y)}
        if z is not None:
            e["z"] = z
        add(e)
    for kind, pts in LINES:
        if kind == "line":
            add({"kind": "line", "layerId": "bordur", "attrs": {}, "a": P(*pts[0]), "b": P(*pts[1])})
        else:
            add({"kind": "polyline", "layerId": "bordur", "attrs": {}, "pts": [P(*p) for p in pts]})
    for name, x, y, z, s in MEASURED:
        attrs = {"Ad": name}
        if s is not None:
            attrs["Hassasiyet"] = s
        e = {"kind": "point", "layerId": "olcum", "attrs": attrs, "p": P(x, y)}
        if z is not None:
            e["z"] = z
        add(e)
    settings = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if work in ("map", "road"):
        settings["road"] = {"surfaces": [{"id": "t-1", "name": "Tasarım", "points": {"layers": ["zemin"]}},
                                         {"id": "t-2", "name": "Boş", "points": {"layers": ["bos"]}}]}
    if work == "gis":
        settings["workspace"] = "gis"
        settings.pop("cadWork")
    return {"format": "kentos.document", "version": 2, "name": "Uygulama", "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": layers, "activeLayer": "0", "entities": ents,
            "uids": [uid(e["id"]) for e in ents], "styles": {"items": [], "categories": []}}


def ids():
    """The persistent ids of the design points, the lines and the measured points, in the drawing's order."""
    n = 121
    design = [uid(n + 1 + i) for i in range(len(DESIGN))]
    n += len(DESIGN)
    lines = [uid(n + 1 + i) for i in range(len(LINES))]
    n += len(LINES)
    measured = [uid(n + 1 + i) for i in range(len(MEASURED))]
    return design, lines, measured


def number(text):
    if text is None:
        return None
    t = text.strip().replace(",", ".")
    body = t[1:] if t.startswith("-") else t
    whole, sep, frac = body.partition(".")
    if not whole.isdigit() or (sep and not frac.isdigit()):
        return None
    return float(t)


def digest_of(design, lines):
    text = ""
    for name, x, y, z in design:
        zt = "-" if z is None else f"{z:.6f}"
        text += f"p\t{name}\t{X0 + x:.6f}\t{Y0 + y:.6f}\t{zt}\n"
    for _, pts in lines:
        text += "l\t" + ";".join(f"{X0 + x:.6f},{Y0 + y:.6f}" for x, y in pts) + "\n"
    return hashlib.sha256(text.encode()).hexdigest()


def digest(with_design=True, with_lines=True):
    return digest_of(DESIGN if with_design else [], LINES if with_lines else [])


# docs/adr/2820: an older design the field was given: P1 3 cm south, P7 not yet there, P8 with a height, the kerb 2 cm
# north. Its points keep the drawing's objects' ids.
OLD_DESIGN = [("P1", 10.0, 9.97, 100.3), ("P2", 20.0, 10.0, 100.5), ("P3", 30.0, 10.0, 100.7), ("P4", 40.0, 10.0, 100.9),
              ("P5", 60.0, 60.0, 101.8), ("P6", 60.6, 60.0, 101.812), ("P8", 90.0, 10.0, 100.1)]
OLD_LINES = [("polyline", [(0.0, 30.02), (100.0, 30.02)]), ("line", [(50.0, 40.0), (50.0, 90.0)])]


def num(v):
    """A number as Rust writes an f64 (the shortest text that reads back the same, no `.0` on a whole one)."""
    t = repr(float(v))
    assert "e" not in t, t
    return t[:-2] if t.endswith(".0") else t


def csv_points(design, uids):
    return "Ad;Y;X;Z;Kimlik\n" + "".join(
        f"{n};{num(X0 + x)};{num(Y0 + y)};{'' if z is None else num(z)};{u}\n" for (n, x, y, z), u in zip(design, uids))


def csv_lines(lines, uids):
    out = "Kimlik;Sıra;Y;X;Bulge\n"
    for (_, pts), u in zip(lines, uids):
        out += "".join(f"{u};{k + 1};{num(X0 + x)};{num(Y0 + y)};{num(0.0)}\n" for k, (x, y) in enumerate(pts))
    return out


def old_ids():
    d_ids, l_ids, _ = ids()
    by_name = {n: u for (n, *_), u in zip(DESIGN, d_ids)}
    return [by_name[n] for n, *_ in OLD_DESIGN], l_ids


def signed(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2))
    d = math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)
    return -d if dx * (p[1] - a[1]) - dy * (p[0] - a[0]) < 0 else d


# ── The field packages (docs/adr/2420) ─────────────────────────────────────────────────────────────────────────────
def package_manifest(digest_, epsg, epoch, heights, acceptance, stamp, revision):
    """`saha-paketi.json` as `cad.field.package` writes it (docs/adr/1920 §1)."""
    return {"format": "kentos.field-package", "version": 1, "project": "Uygulama", "stamp": stamp,
            "revision": revision, "designDigest": digest_,
            "system": {"epsg": epsg, "name": None if epsg is None else "TUREF / TM36" if epsg == "EPSG:5256" else epsg},
            "epoch": epoch, "heights": heights, "acceptance": acceptance, "notes": ["Kontrol noktası K1'den başlayın."],
            "surface": None, "points": len(DESIGN), "lines": len(LINES),
            "files": ["saha-paketi.json", "noktalar.csv", "cizgiler.csv"]}


# The package of this design (the limits FULL gives), and one of an older design in another system.
CURRENT = package_manifest(None, "EPSG:5256", None, None, {"horizontal": 0.02, "vertical": 0.03, "quality": 0.05},
                           "2026-11-08T09:00:00", "12")
OLDER = package_manifest("0" * 64, "EPSG:2320", 2005.0, "Ortometrik yükseklik", {"horizontal": 0.06},
                         "2026-10-01T08:30:00", "7")


def zipped(entries):
    """A deterministic .zip of (name, text) entries: a fixed date, stored (no zlib version in the bytes)."""
    import io
    import zipfile
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        for name, text in entries:
            info = zipfile.ZipInfo(name, date_time=(2026, 11, 8, 9, 0, 0))
            info.compress_type = zipfile.ZIP_STORED
            z.writestr(info, text.encode("utf-8"))
    return buf.getvalue()


# The package of the older design (docs/adr/2820): the project's system, FULL's limits.
OLD_DESIGN_PACKAGE = dict(package_manifest(digest_of(OLD_DESIGN, OLD_LINES), "EPSG:5256", None, None,
                                           {"horizontal": 0.02, "vertical": 0.03, "quality": 0.05},
                                           "2026-10-20T10:00:00", "9"), points=len(OLD_DESIGN), lines=len(OLD_LINES))


# The package that carries its surface (docs/adr/3220): the current design, PG as `yuzey.xml`; the sources' digest of
# the surface it was made of is not the drawing's now (“0…0”).
SURFACE_XML = tin_xml(PG_CORNERS, PG_FACES, PG)


def surface_manifest():
    return dict(CURRENT, designDigest=digest(),
                surface={"id": "t-1", "name": "Tasarım", "digest": "0" * 64, "file": "yuzey.xml",
                         "vertices": len(PG_CORNERS), "triangles": len(PG_FACES),
                         "sha256": hashlib.sha256(SURFACE_XML.encode("utf-8")).hexdigest()},
                files=["saha-paketi.json", "noktalar.csv", "cizgiler.csv", "yuzey.xml"])


def package_files():
    """The fixture files: the current package, the older one, and a zip that is not a package; the design's files as
    `cad.field.package` writes them (docs/adr/1920). docs/adr/2820: the older design's package, and the current one
    with P1 moved 1 cm in its noktalar.csv (its files no longer its digest's). docs/adr/3220: the package with its
    surface, the same with its first corner 1 cm higher in yuzey.xml (no longer its SHA-256's), and one whose manifest
    names a yuzey.xml it does not hold."""
    d_ids, l_ids, _ = ids()
    points, strings = csv_points(DESIGN, d_ids), csv_lines(LINES, l_ids)
    current = dict(CURRENT, designDigest=digest())
    moved = [("P1", 10.01, 10.0, 100.3)] + DESIGN[1:]
    o_ids, o_lines = old_ids()
    def entries(m, pts, lns):
        return [("saha-paketi.json", json.dumps(m, ensure_ascii=False, indent=2)), ("noktalar.csv", pts),
                ("cizgiler.csv", lns)]
    pack = lambda m, pts, lns: zipped(entries(m, pts, lns))  # noqa: E731
    raised = tin_xml(PG_CORNERS, PG_FACES, lambda x, y: PG(x, y) + (0.01 if (x, y) == PG_CORNERS[0] else 0.0))
    return {
        "saha-paketi.zip": pack(current, points, strings),
        "eski-paket.zip": pack(OLDER, points, strings),
        "paket-degil.zip": zipped([("notlar.txt", "Saha notları\n")]),
        "eski-tasarim-paketi.zip": pack(OLD_DESIGN_PACKAGE, csv_points(OLD_DESIGN, o_ids), csv_lines(OLD_LINES, o_lines)),
        "bozuk-tasarim-paketi.zip": pack(current, csv_points(moved, d_ids), strings),
        "yuzeyli-paket.zip": zipped(entries(surface_manifest(), points, strings) + [("yuzey.xml", SURFACE_XML)]),
        "bozuk-yuzey-paketi.zip": zipped(entries(surface_manifest(), points, strings) + [("yuzey.xml", raised)]),
        "yuzeyi-eksik-paket.zip": zipped(entries(surface_manifest(), points, strings)),
    }


def package_block(manifest, i, path):
    """The answer's `package`: the file, its stamp, revision and digest, its system against the project's (EPSG:5256,
    no epoch, no heights), its limits and which the check took (those the input did not give; quality only with
    qualityField)."""
    acc = manifest["acceptance"]
    out = {"path": path, "stamp": manifest["stamp"], "revision": manifest["revision"],
           "digest": manifest["designDigest"]}
    if manifest["system"]["epsg"] is not None:
        out["epsg"] = manifest["system"]["epsg"]
    if manifest["epoch"] is not None:
        out["epoch"] = manifest["epoch"]
    if manifest["heights"] is not None:
        out["heights"] = manifest["heights"]
    out["sameSystem"] = manifest["system"]["epsg"] == "EPSG:5256"
    out["sameEpoch"] = manifest["epoch"] is None
    out["sameHeights"] = manifest["heights"] is None
    for k in ("horizontal", "vertical", "quality"):
        if k in acc:
            out[k] = acc[k]
    taken = [k for k in ("horizontal", "vertical", "quality") if k in acc and k not in i
             and (k != "quality" or "qualityField" in i)]
    if taken:
        out["taken"] = taken
    return out


def report(design=True, surface=True, lines=True, search=None, horizontal=None, vertical=None, quality=None,
           expected=None, quality_field=True, plan=None, ground=G):
    """`plan`: the design compared, (points, lines, their ids), the package's (docs/adr/2820); none: the drawing's.
    `ground`: the surface compared, the drawing's G or the package's PG (docs/adr/3220)."""
    d_ids, l_ids, m_ids = ids()
    DESIGN_, LINES_ = DESIGN, LINES
    if plan is not None:
        DESIGN_, LINES_, (d_ids, l_ids) = plan[0], plan[1], plan[2]
    names = [n for n, *_ in DESIGN_] if design else []
    claimed = set()
    for name, *_ in MEASURED:
        hits = [i for i, n in enumerate(names) if n == name]
        if len(hits) == 1:
            claimed.add(hits[0])
    used = set()
    out, counts = [], {"within": 0, "outside": 0, "unmatched": 0, "ambiguous": 0}
    low = 0
    hs, vs = [], []
    for k, (name, x, y, z, s) in enumerate(MEASURED):
        flags = []
        hits = [i for i, n in enumerate(names) if n == name] if name else []
        match, by, amb = None, None, False
        if len(hits) == 1:
            match, by = hits[0], "name"
        elif len(hits) > 1:
            amb = True
        elif search is not None and design:
            near = [i for i, (_, dx, dy, _) in enumerate(DESIGN_) if i not in claimed and math.hypot(dx - x, dy - y) <= search]
            if len(near) == 1:
                match, by = near[0], "place"
                flags.append("proposed")
            elif len(near) > 1:
                amb = True
        sigma = number(s) if quality_field else None
        row = {"uid": m_ids[k], "name": name}
        outside = False
        if match is not None:
            used.add(match)
            dn_, dx, dy, dz = DESIGN_[match]
            de, dn = x - dx, y - dy
            h = math.hypot(de, dn)
            row.update({"designUid": d_ids[match], "designName": dn_, "by": by, "de": de, "dn": dn, "horizontal": h})
            hs.append(h)
            if horizontal is not None and h > horizontal:
                flags.append("outsideHorizontal")
                outside = True
            if z is not None and dz is not None:
                dh = z - dz
                row["dh"] = dh
                vs.append(dh)
                if vertical is not None and abs(dh) > vertical:
                    flags.append("outsideVertical")
                    outside = True
        if surface:
            g = ground(x, y)
            if g is None:
                flags.append("offSurface")
            elif z is not None:
                row["surface"] = z - g
                if vertical is not None and abs(z - g) > vertical:
                    flags.append("outsideSurface")
                    outside = True
        on_line = False
        if lines and match is None and not amb:
            best = None
            for li, (_, pts) in enumerate(LINES_):
                for a, b in zip(pts, pts[1:]):
                    d = signed((x, y), a, b)
                    if best is None or abs(d) < abs(best[0]):
                        best = (d, li)
            if search is None or abs(best[0]) <= search:
                on_line = True
                row["normal"] = best[0]
                row["lineUid"] = l_ids[best[1]]
                if horizontal is not None and abs(best[0]) > horizontal:
                    flags.append("outsideLine")
                    outside = True
        if sigma is not None:
            row["sigma"] = sigma
        if quality is not None and sigma is not None and sigma > quality:
            flags.append("lowQuality")
            low += 1
        if z is None and (surface or (match is not None and DESIGN_[match][3] is not None)):
            flags.append("noHeight")
        only_surface = not design and not lines
        if design and amb:
            status = "ambiguous"
        elif match is None and not on_line and not only_surface:
            status = "unmatched"
        elif outside:
            status = "outside"
        else:
            status = "within"
        counts[status] += 1
        row["status"] = status
        row["flags"] = sorted(flags, key=FLAGS.index)
        out.append(row)
    missing = [{"uid": d_ids[i], "name": DESIGN_[i][0]} for i in range(len(DESIGN_)) if design and i not in used]
    dig = digest_of(DESIGN_ if design else [], LINES_ if lines else [])
    result = {"designDigest": dig, "points": out, "missing": missing, **counts, "lowQuality": low}
    if expected is not None and expected != dig:
        result["designChanged"] = True
    if hs:
        result["rmsHorizontal"] = math.sqrt(sum(h * h for h in hs) / len(hs))
        result["maxHorizontal"] = max(hs)
    if vs:
        result["rmsVertical"] = math.sqrt(sum(v * v for v in vs) / len(vs))
        result["maxVertical"] = max(abs(v) for v in vs)
    return result


# ── The cases ─────────────────────────────────────────────────────────────────────────────────────────────────────
SAME = {"revision": "same", "canUndo": False}
FULL = {"measured": "olcum", "design": "tasarim", "surface": "Tasarım", "lines": "bordur", "search": 0.5,
        "horizontal": 0.02, "vertical": 0.03, "qualityField": "Hassasiyet", "quality": 0.05}
NOT_MAP = ("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim "
                                 "ikisinden biri değil.")


def inp(**over):
    d = dict(FULL)
    for k, v in over.items():
        if v is None:
            d.pop(k, None)
        else:
            d[k] = v
    return d


def answer(i):
    pkg = i.get("package")
    if i.get("packageDesign"):
        return package_design_answer(i)
    if pkg is None:
        return report(design="design" in i, surface="surface" in i, lines="lines" in i, search=i.get("search"),
                      horizontal=i.get("horizontal"), vertical=i.get("vertical"), quality=i.get("quality"),
                      expected=i.get("expectedDesign"), quality_field="qualityField" in i)
    carries = pkg.endswith("yuzeyli-paket.zip")
    manifest = (surface_manifest() if carries else dict(CURRENT, designDigest=digest()) if pkg.endswith("saha-paketi.zip")
                else OLDER)
    acc = manifest["acceptance"]
    pick = lambda k: i.get(k, acc.get(k)) if k != "quality" or "qualityField" in i else i.get(k)
    out = report(design="design" in i, surface="surface" in i, lines="lines" in i, search=i.get("search"),
                 horizontal=pick("horizontal"), vertical=pick("vertical"), quality=pick("quality"),
                 expected=manifest["designDigest"], quality_field="qualityField" in i)
    # docs/adr/3220: without the package's design the drawing's surface is compared; the package names the surface it
    # was made of, whose sources' digest is not the drawing's now.
    if carries and "surface" in i:
        out["surfaceChanged"] = True
    out["package"] = package_block(manifest, i, pkg)
    return out


# The answer's keys in the contract's order: the digest and the changes first.
ORDER = ["designDigest", "designChanged", "surfaceChanged", "points", "missing", "within", "outside", "unmatched",
         "ambiguous", "lowQuality", "rmsHorizontal", "rmsVertical", "maxHorizontal", "maxVertical", "package"]


def order(out):
    return {k: out[k] for k in ORDER if k in out}


def package_design_answer(i):
    """docs/adr/2820: the package's own design compared; the drawing's layers given only say whether it changed."""
    pkg = i["package"]
    old = pkg.endswith("eski-tasarim-paketi.zip")
    # docs/adr/3220: the package that carries its surface has it compared, the surface given or not.
    carries = pkg.endswith("yuzeyli-paket.zip")
    manifest = (OLD_DESIGN_PACKAGE if old else surface_manifest() if carries
                else dict(CURRENT, designDigest=digest(), **i.get("$written", {})))
    plan = (OLD_DESIGN, OLD_LINES, old_ids()) if old else (DESIGN, LINES, ids()[:2])
    acc = manifest["acceptance"]
    pick = lambda k: i.get(k, acc.get(k)) if k != "quality" or "qualityField" in i else i.get(k)  # noqa: E731
    out = report(design=True, surface="surface" in i or carries, lines=True, search=i.get("search"),
                 horizontal=pick("horizontal"), vertical=pick("vertical"), quality=pick("quality"),
                 quality_field="qualityField" in i, plan=plan, ground=PG if carries else G)
    assert out["designDigest"] == manifest["designDigest"]
    out.pop("designChanged", None)
    if ("design" in i or "lines" in i) and digest("design" in i, "lines" in i) != manifest["designDigest"]:
        out["designChanged"] = True
    if carries and "surface" in i:
        out["surfaceChanged"] = True
    block = package_block(manifest, i, pkg)
    block["design"] = True
    if carries or "$written" in i:
        block["surface"] = True
    out["package"] = block
    return order(out)


def ok(name, i, op="plan", setup_=None):
    step = {"op": op, "input": i, "result": {"status": "completed", "output": None if op == "validate" else answer(i),
                                             "warnings": []}, "expect": SAME, "within": W}
    case = {"name": name, "steps": [step]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def refused(name, i, code, message, path_=None, setup_=None):
    err = {"code": code, "message": message}
    if path_ is not None:
        err["path"] = path_
    case = {"name": name, "steps": [{"op": "execute", "input": i, "result": {"status": "failed", "error": err},
                                     "expect": SAME}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def cases():
    full = answer(FULL)
    st = {p["name"]: (p["status"], p["flags"]) for p in full["points"]}
    assert st["P1"] == ("within", []), st["P1"]
    assert st["P2"][0] == "outside" and "outsideHorizontal" in st["P2"][1]
    assert "outsideVertical" in st["P3"][1] and "outsideSurface" in st["P3"][1]
    assert st["p4x"][1][0] == "proposed"
    assert st["X9"][0] == "ambiguous"
    assert st["R1"][0] == "unmatched" and st["S1"][1] == ["noHeight"]
    assert st["B1"] == ("outside", ["outsideLine"]) and st["B2"] == ("within", []), (st["B1"], st["B2"])
    assert "lowQuality" in st["P8"][1] and st["P8"][0] == "within"
    assert [m["name"] for m in full["missing"]] == ["P5", "P6", "P7"]
    bad = "invalid_asbuilt_check"
    return [
        ok("tam: noktalar adla ve yerle, yüzey, bordür; toleranslar ve kalite sınırı", FULL),
        ok("uygula: salt okunur, çizim değişmez", FULL, op="execute"),
        ok("doğrulama", FULL, op="validate"),
        ok("yalnız adla: yarıçapsız, öneri ve belirsiz yok", inp(search=None)),
        ok("tasarım noktası yok: yüzey ve bordür, durum toleranstan", inp(design=None, search=None)),
        ok("toleranssız: yalnız farklar", inp(horizontal=None, vertical=None, quality=None, qualityField=None)),
        ok("yalnız bordür, 0,5 m içindekiler", {"measured": "olcum", "lines": "bordur", "horizontal": 0.02, "search": 0.5}),
        ok("yalnız bordür, yarıçapsız: her nokta en yakın çizgiye", {"measured": "olcum", "lines": "bordur"}),
        ok("yalnız yüzey: her nokta yüzeye", {"measured": "olcum", "surface": "Tasarım", "vertical": 0.03}),
        ok("aynı tasarım özeti: değişmedi", inp(expectedDesign=digest())),
        ok("başka tasarım özeti: tasarım değişti, yine karşılaştırılır", inp(expectedDesign="0" * 64)),
        ok("CBS projesinde yüzeysiz", inp(surface=None), setup_=setup("gis")),
        ok("Yol projesinde", FULL, setup_=setup("road")),
        refused("tasarım yok", {"measured": "olcum"}, bad, "Karşılaştırılacak tasarım yok: tasarım noktalarının "
                "katmanını, yüzeyi ya da tasarım çizgilerinin katmanını verin.", "design"),
        refused("sıfır arama yarıçapı", inp(search=0.0), bad,
                "Arama yarıçapı 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.", "search"),
        refused("eksi yatay tolerans", inp(horizontal=-0.01), bad,
                "Yatay tolerans 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.", "horizontal"),
        refused("sıfır düşey tolerans", inp(vertical=0.0), bad,
                "Düşey tolerans 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.", "vertical"),
        refused("boş Ad özniteliği", inp(field=" "), bad,
                "Ad özniteliği boş; özniteliğin adını yazın ya da boş bırakın (Ad).", "field"),
        refused("boş kalite özniteliği", inp(qualityField=""), bad,
                "Kalite özniteliği boş; σ'nın yazıldığı özniteliğin adını yazın.", "qualityField"),
        refused("kalite sınırı özniteliksiz", inp(qualityField=None), bad,
                "Kalite sınırı verildi ama σ'nın özniteliği yok; qualityField'ı da verin.", "qualityField"),
        refused("olmayan ölçülen katman", inp(measured="yok"), "layer_not_found", "“yok” kimlikli katman çizimde yok.",
                "measured"),
        refused("olmayan tasarım katmanı", inp(design="yok"), "layer_not_found", "“yok” kimlikli katman çizimde yok.",
                "design"),
        refused("olmayan bordür katmanı", inp(lines="yok"), "layer_not_found", "“yok” kimlikli katman çizimde yok.",
                "lines"),
        refused("olmayan yüzey", inp(surface="Kazı"), "unknown_terrain", "“Kazı” kimlikli ya da adlı yüzey projede yok.",
                "surface"),
        refused("kurulamayan yüzey", inp(surface="Boş"), "terrain_unbuildable", "$any", None),
        refused("CBS projesinde yüzey", FULL, NOT_MAP[0], NOT_MAP[1], None, setup_=setup("gis")),
    ] + package_cases()


PKG = "$fixture:asbuilt/v1/files/saha-paketi.zip"
OLD_PKG = "$fixture:asbuilt/v1/files/eski-paket.zip"
OLD_DESIGN_PKG = "$fixture:asbuilt/v1/files/eski-tasarim-paketi.zip"
BROKEN_PKG = "$fixture:asbuilt/v1/files/bozuk-tasarim-paketi.zip"


def package_cases():
    """docs/adr/2420: the record checked with the field package it was made with."""
    same = answer(inp(horizontal=None, vertical=None, quality=None, package=PKG))
    full = answer(FULL)
    assert {k: v for k, v in same.items() if k != "package"} == full, "the package's limits are FULL's"
    older = answer(inp(horizontal=None, vertical=None, quality=None, package=OLD_PKG))
    assert older["designChanged"] and not older["package"]["sameSystem"]
    bad = "invalid_asbuilt_check"
    return [
        ok("paket: tasarım özeti ve kabul sınırları paketten", inp(horizontal=None, vertical=None, quality=None,
                                                                   package=PKG)),
        ok("paket: girdinin sınırı paketinkinden önce gelir", inp(horizontal=0.06, vertical=None, quality=None,
                                                                  package=PKG)),
        ok("paket: σ özniteliği olmadan kalite sınırı alınmaz", inp(horizontal=None, vertical=None, quality=None,
                                                                    qualityField=None, package=PKG)),
        ok("paket: eski tasarımın paketi, başka sistem, epok ve kot", inp(horizontal=None, vertical=None, quality=None,
                                                                         package=OLD_PKG)),
        ok("paket: aynı beklenen özetle", inp(package=PKG, expectedDesign=digest())),
        refused("paket: göreli yol", inp(package="saha-paketi.zip"), "invalid_file_path",
                "Saha paketinin yolu boş ya da göreli; mutlak bir .zip yolu verin.", "package"),
        refused("paket: .zip değil", inp(package="$fixture:asbuilt/v1/files/saha-paketi.json"), "invalid_file_path",
                "$any", "package"),
        refused("paket: olmayan dosya", inp(package="$fixture:asbuilt/v1/files/yok.zip"), "file_not_found", "$any",
                "package"),
        refused("paket: saha paketi olmayan zip", inp(package="$fixture:asbuilt/v1/files/paket-degil.zip"), bad, "$any",
                "package"),
        refused("paket: beklenen özet paketinkinden farklı", inp(package=PKG, expectedDesign="0" * 64), bad,
                "Beklenen tasarım özeti saha paketininkinden farklı; yalnız paketi ya da yalnız özeti verin.",
                "expectedDesign"),
    ] + package_design_cases()


def package_design_cases():
    """docs/adr/2820: the record checked with its package's own design."""
    bare = dict(horizontal=None, vertical=None, quality=None)
    same = answer(inp(**bare, package=PKG, packageDesign=True))
    full = answer(FULL)
    assert {k: v for k, v in same.items() if k != "package"} == full, "the package's design is the drawing's"
    old = answer(inp(**bare, package=OLD_DESIGN_PKG, packageDesign=True))
    assert old["designChanged"] and old["designDigest"] != full["designDigest"]
    p1 = next(p for p in old["points"] if p["name"] == "P1")
    assert abs(p1["dn"] - 0.03) < 1e-9, p1
    assert [m["name"] for m in old["missing"]] == ["P5", "P6"], old["missing"]
    layerless = {"measured": "olcum", "surface": "Tasarım", "search": 0.5, "qualityField": "Hassasiyet",
                 "package": OLD_DESIGN_PKG, "packageDesign": True}
    assert "designChanged" not in answer(layerless)
    bad = "invalid_asbuilt_check"
    return [
        ok("paketin tasarımı: değişmemiş tasarım, katmanınkiyle aynı sonuç", inp(**bare, package=PKG,
                                                                                packageDesign=True)),
        ok("paketin tasarımı: eski tasarım; kayıt sahaya verilenle, çizimdeki tasarım değişti",
           inp(**bare, package=OLD_DESIGN_PKG, packageDesign=True)),
        ok("paketin tasarımı katmansız: değişim söylenmez", layerless),
        refused("paketin tasarımı paketsiz", inp(packageDesign=True), bad,
                "Paketin tasarımıyla denetlemek için kaydın saha paketini (package) de verin.", "packageDesign"),
        refused("paketin tasarımı: dosyaları özetiyle tutmuyor", inp(package=BROKEN_PKG, packageDesign=True), bad,
                "“bozuk-tasarim-paketi.zip” paketinin tasarım dosyaları (noktalar.csv, cizgiler.csv) özetiyle tutmuyor; "
                "paket bozulmuş ya da elle değiştirilmiş. Paketi Saha paketi yaz… ile yeniden yazın.", "package"),
        refused("paketin tasarımı: başka özetli eski paket", inp(package=OLD_PKG, packageDesign=True), bad,
                "“eski-paket.zip” paketinin tasarım dosyaları (noktalar.csv, cizgiler.csv) özetiyle tutmuyor; paket "
                "bozulmuş ya da elle değiştirilmiş. Paketi Saha paketi yaz… ile yeniden yazın.", "package"),
    ] + package_surface_cases()


SURFACE_PKG = "$fixture:asbuilt/v1/files/yuzeyli-paket.zip"
# The package `cad.field.package` writes in the case's first step (docs/adr/3220): the drawing's surface carried.
WRITTEN = "$tmp:asbuilt-yuzeyli-saha.zip"
WRITTEN_STAMP = "2026-11-08T09:00:00"


def package_surface_cases():
    """docs/adr/3220: the record checked with its package's own surface."""
    bare = dict(horizontal=None, vertical=None, quality=None)
    carried = answer(inp(**bare, surface=None, package=SURFACE_PKG, packageDesign=True))
    by_name = {p["name"]: p for p in carried["points"]}
    # P1 measured 100.31 at (10.01, 10): the drawing's plane gives 100.3002, the package's 5 cm lower.
    assert abs(by_name["P1"]["surface"] - (100.31 - (100.0 + 0.02 * 10.01 + 0.01 * 10.0 - 0.05))) < 1e-12
    assert "outsideSurface" in by_name["P1"]["flags"] and "offSurface" in by_name["X9"]["flags"]
    assert "surfaceChanged" not in carried and carried["package"]["surface"]
    changed = answer(inp(**bare, package=SURFACE_PKG, packageDesign=True))
    assert changed["surfaceChanged"] and changed["points"] == carried["points"]
    plain = answer(inp(**bare, package=SURFACE_PKG))
    assert plain["surfaceChanged"] and "surface" not in plain["package"]
    assert plain["points"] == answer(inp(**bare, package=PKG))["points"], "the drawing's surface without the package's design"
    # The package written in the step before carries the drawing's own surface: the same as with the layers.
    write = {"op": "execute", "command": "cad.field.package",
             "input": {"design": "tasarim", "lines": "bordur", "surface": "Tasarım", "horizontal": 0.02, "vertical": 0.03,
                       "quality": 0.05, "notes": ["Kontrol noktası K1'den başlayın."], "path": WRITTEN,
                       "stamp": WRITTEN_STAMP},
             "result": {"status": "completed", "output": "$any", "warnings": []}, "expect": SAME}
    i = inp(**bare, package=WRITTEN, packageDesign=True)
    same = package_design_answer(dict(i, **{"$written": {"stamp": WRITTEN_STAMP, "revision": "$current"}}))
    assert {k: v for k, v in same.items() if k != "package"} == answer(FULL), "the package's surface is the drawing's"
    check = {"op": "plan", "input": i, "result": {"status": "completed", "output": same, "warnings": []}, "expect": SAME,
             "within": W}
    bad = "invalid_asbuilt_check"
    return [
        ok("paketin yüzeyi: kayıt paketin taşıdığı yüzeyle", inp(**bare, surface=None, package=SURFACE_PKG,
                                                                  packageDesign=True)),
        ok("paketin yüzeyi: çizimdeki yüzey paketinkinden sonra değişmiş", inp(**bare, package=SURFACE_PKG,
                                                                               packageDesign=True)),
        ok("paket: yüzeyin değiştiği söylenir, paketin tasarımı istenmeden çizimin yüzeyiyle", inp(**bare,
                                                                                                   package=SURFACE_PKG)),
        {"name": "paketin yüzeyi: Saha paketi yaz… ile yazılan paketin yüzeyi çizimin yüzeyi", "steps": [write, check]},
        refused("paketin yüzeyi: yuzey.xml özetiyle tutmuyor", inp(package="$fixture:asbuilt/v1/files/bozuk-yuzey-paketi.zip",
                                                                   packageDesign=True), bad,
                "“bozuk-yuzey-paketi.zip” paketinin yüzeyi (yuzey.xml) özetiyle tutmuyor; paket bozulmuş ya da elle "
                "değiştirilmiş. Paketi Saha paketi yaz… ile yeniden yazın.", "package"),
        refused("paketin yüzeyi: pakette yuzey.xml yok", inp(package="$fixture:asbuilt/v1/files/yuzeyi-eksik-paket.zip",
                                                             packageDesign=True), bad,
                "“yuzeyi-eksik-paket.zip” paketinin yüzeyi (yuzey.xml) okunamadı: pakette yuzey.xml yok.", "package"),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.asbuilt.check", "commandVersion": 1,
           "title": "Uygulama kontrolü",
           "note": "ADR 1700. Bağımsız başvuru scripts/fixtures/asbuilt_cases.py: tasarım yüzeyi düzlem, eşleme, farklar, "
                   "işaretli dik uzaklık, durum ve özet elle; KentOS kodu yok. Toleranslar durumların kendi değerleridir. "
                   "ADR 2420: saha paketleri fixtures/asbuilt/v1/files altında Python'un zipfile'ıyla yazılır.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    files = package_files()
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text and all(
            (PACKAGES / n).exists() and (PACKAGES / n).read_bytes() == b for n, b in files.items())
        print("Uygulama kontrolü ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    PACKAGES.mkdir(parents=True, exist_ok=True)
    for n, b in files.items():
        (PACKAGES / n).write_bytes(b)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
