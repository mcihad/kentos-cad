#!/usr/bin/env python3
"""The shared cases of a part's write that takes the rows outside its area along (docs/adr/0531, 0585, plan/gis.md
CBS-12): which rows follow the changed corners, and the full validity rule that keeps a strict write from leaving a
row invalid.

    /usr/bin/python3 scripts/fixtures/source_neighbours_cases.py --files   # GDAL writes fixtures/geopackage/v1/komsu-gecerlilik.gpkg
    /usr/bin/python3 scripts/fixtures/source_neighbours_cases.py           # writes fixtures/commands/v1/cad.sources.sync.neighbours.json
    /usr/bin/python3 scripts/fixtures/source_neighbours_cases.py --check   # compares

The file's rows are written by GDAL's OGR: a parcel A in the area and three rows outside it sharing its corners. B a
quadrangle under A's right edge, C a triangle on it, D a bow-tie (already invalid) at A's top right corner. A case moves
or removes a corner of A and writes the part. Without KentOS code:

- which rows follow: ADR 0160's rule (a moved corner takes every row's vertex at its place; a removed one the vertex
  whose ring runs prev – at – next through it, a ring left with fewer than three vertices invalid), written out here;
- whether a row is invalid: GEOS's IsValid through GDAL's OGR, before and after. A row that was valid and is not after
  following is newly invalid: a strict write refuses, a loose one leaves it. A row invalid before keeps following.

A write to the file runs on a `$tmp:` copy; what it wrote is read back by cad.sources.query (WKT in the project's
system, its numbers as Python's shortest repr). A row's lasting id is the importer's (uuid5 of the table and the key).
"""
import json
import sys
import uuid
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import bound_sources_cases as bs  # noqa: E402
import gpkg_import_cases as gi  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
NAME = "komsu-gecerlilik.gpkg"
FILE = ROOT / "fixtures/geopackage/v1" / NAME
OUT = ROOT / "fixtures/commands/v1/cad.sources.sync.neighbours.json"
SRID = 5256
ANY = "$any"
X0, Y0 = 487000.0, 4420000.0
# Only A's box meets the area; the others start at x = 10.
AREA = [X0 - 10.0, Y0 - 10.0, X0 + 5.0, Y0 + 30.0]

# The rows in key order, in metres from (X0, Y0).
ROWS = [
    ("A", [(0, 0), (10, 0), (10, 4), (10, 6), (10, 10), (0, 10)]),
    ("B", [(10, 0), (20, 0), (20, 4), (10, 4)]),
    ("C", [(10, 6), (14, 8), (10, 10)]),
    ("D", [(10, 10), (20, 20), (20, 10), (10, 20)]),
]


def world(ring):
    return [(X0 + x, Y0 + y) for x, y in ring]


def write_files():
    from osgeo import ogr, osr
    if FILE.exists():
        FILE.unlink()
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(SRID)
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(FILE))
    lyr = ds.CreateLayer("parsel", srs=srs, geom_type=ogr.wkbPolygon)
    lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    for ad, ring in ROWS:
        f = ogr.Feature(lyr.GetLayerDefn())
        pts = world(ring)
        f.SetGeometry(ogr.CreateGeometryFromWkt("POLYGON ((" + ", ".join(f"{x} {y}" for x, y in [*pts, pts[0]])
                                                + "))"))
        f.SetField("ad", ad)
        lyr.CreateFeature(f)
    ds = None


def uid_of(ad):
    fid = next(i for i, (a, _) in enumerate(ROWS, 1) if a == ad)
    return str(uuid.uuid5(gi.NAMESPACE, f"gpkg:parsel:{fid}"))


def valid(ring):
    """GEOS's validity of a ring (in metres from the origin; the offset does not change it)."""
    from osgeo import gdal, ogr
    gdal.PushErrorHandler("CPLQuietErrorHandler")
    if len(ring) < 3:
        return False
    pts = world(ring)
    g = ogr.CreateGeometryFromWkt("POLYGON ((" + ", ".join(f"{x!r} {y!r}" for x, y in [*pts, pts[0]]) + "))")
    return g.IsValid()


def same(p, q):
    return abs(p[0] - q[0]) <= 1e-9 and abs(p[1] - q[1]) <= 1e-9


def follow(change):
    """ADR 0160's rule on B, C and D: {name: ring after} for the rows the change reaches, and those invalid by too few
    vertices (as None)."""
    out = {}
    for ad, ring in ROWS[1:]:
        if change[0] == "move":
            moves = change[1]
            if any(same(v, f) for v in ring for f, _ in moves):
                out[ad] = [next((t for f, t in moves if same(v, f)), v) for v in ring]
            continue
        _, at, prev, nxt = change
        n = len(ring)
        hit = [k for k in range(n) if same(ring[k], at)
               and ((same(ring[k - 1], prev) and same(ring[(k + 1) % n], nxt))
                    or (same(ring[k - 1], nxt) and same(ring[(k + 1) % n], prev)))]
        if hit:
            out[ad] = None if n - 1 < 3 else [v for k, v in enumerate(ring) if k != hit[0]]
    return out


def verdict(change):
    """(followed rows by name, left names): a row newly invalid, or left with too few vertices, is left."""
    followed, left = {}, []
    before = dict(ROWS)
    for ad, ring in follow(change).items():
        if ring is None or (valid(before[ad]) and not valid(ring)):
            left.append(ad)
        else:
            followed[ad] = ring
    return followed, left


def num(v):
    return repr(float(v)).removesuffix(".0") if float(v).is_integer() else repr(float(v))


def wkt(ring):
    pts = world(ring)
    return "POLYGON ((" + ", ".join(f"{num(x)} {num(y)}" for x, y in [*pts, pts[0]]) + "))"


# ── steps ────────────────────────────────────────────────────────────────────

def link(path):
    return bs.step({"edit": {"kind": "link", "path": path, "table": "parsel", "extent": AREA}},
                   {"status": "completed", "output": ANY, "warnings": ANY}, {"revision": "changed"},
                   command="cad.sources.edit")


def link_whole(path):
    return bs.step({"edit": {"kind": "link", "path": path, "table": "parsel"}},
                   {"status": "completed", "output": ANY, "warnings": ANY}, {"revision": "changed"},
                   command="cad.sources.edit")


def stretch(moves):
    """A's corners in `moves` (from, to; all by one shift) moved by Esnet's window round them."""
    (fx, fy), (tx, ty) = moves[0]
    dx, dy = tx - fx, ty - fy
    assert all(abs((t[0] - f[0]) - dx) < 1e-12 and abs((t[1] - f[1]) - dy) < 1e-12 for f, t in moves)
    xs = [f[0] for f, _ in moves]
    ys = [f[1] for f, _ in moves]
    return bs.other("cad.entities.stretch", {
        "window": {"min": gi.P(X0 + min(xs) - 0.5, Y0 + min(ys) - 0.5),
                   "max": gi.P(X0 + max(xs) + 0.5, Y0 + max(ys) + 0.5)},
        "dx": dx, "dy": dy, "uids": [uid_of("A")]})


def remove(at):
    ring = ROWS[0][1]
    k = ring.index(at)
    return bs.other("cad.entities.vertex", {"uid": uid_of("A"), "action": "remove",
                                            "place": {"kind": "vertex", "part": 0, "ring": 0, "index": k}})


def change_of(edit):
    """The change ADR 0160's rule follows."""
    if edit[0] == "move":
        return ("move", edit[1])
    ring = ROWS[0][1]
    k = ring.index(edit[1])
    return ("remove", edit[1], ring[k - 1], ring[(k + 1) % len(ring)])


def edit_step(edit):
    return stretch(edit[1]) if edit[0] == "move" else remove(edit[1])


LEFT_TEXT = "Alanın dışındaki {} komşu kayıt ortak köşeyi izleyince geçersiz kalacağı için olduğu gibi kaldı."
STRICT_TEXT = ("Alanın dışındaki {} komşu kayıt ortak köşeyi izleyince geçersiz kalırdı; hiçbir şey yazılmadı. "
               "Alanı genişletip komşuları da çalışma kopyasına alın ya da gevşek kipte (loose) yazın.")


def write(edit, topology, op, part=True):
    """The write and its verdict."""
    inp = {"layer": "import-parsel", "direction": "write"}
    if topology:
        inp["topology"] = topology
    followed, left = verdict(change_of(edit)) if (topology and part) else ({}, [])
    if left and topology == "strict":
        return bs.step(inp, bs.failed("outside_neighbour_invalid", STRICT_TEXT.format(len(left)), "topology"), op=op)
    out = {"added": 0, "modified": 1, "removed": 0, "source": ANY, "revision": "$current"}
    if followed:
        out["followed"] = len(followed)
    if left:
        out["left"] = len(left)
    warnings = [{"code": "neighbours_left", "message": LEFT_TEXT.format(len(left)), "path": "topology"}] if left else []
    return bs.step(inp, bs.done(out, warnings), {"revision": "same" if op == "plan" else "changed"}, op=op)


def read_back(path, ad, ring):
    """cad.sources.query on the file: the row as the file now holds it."""
    fid = next(i for i, (a, _) in enumerate(ROWS, 1) if a == ad)
    out = {"source": ANY, "rows": [{"uid": uid_of(ad), "key": fid, "attrs": {"ad": ad}, "wkt": wkt(ring)}],
           "plan": ANY, "partial": False, "read": 1, "stamp": ANY, "revision": "$current"}
    return bs.step({"path": path, "table": "parsel", "filter": f"ad = '{ad}'", "fields": ["ad"], "geometry": True},
                   bs.done(out), command="cad.sources.query")


def after_write(edit, topology):
    """Each row's ring in the file after the write: A's change; the neighbours that followed; the rest as written."""
    before = dict(ROWS)
    rings = dict(before)
    followed, left = verdict(change_of(edit))
    if topology == "strict" and left:
        return rings
    ring = list(before["A"])
    if edit[0] == "move":
        rings["A"] = [next((t for f, t in edit[1] if same(v, f)), v) for v in ring]
    else:
        rings["A"] = [v for v in ring if not same(v, edit[1])]
    rings.update(followed)
    return rings


# ── the edits ────────────────────────────────────────────────────────────────

B_SHIFT = ("move", [((10, 4), (11, 4))])
B_BOWTIE = ("move", [((10, 4), (25, 2))])
C_FLAT = ("move", [((10, 6), (18, 6))])
C_SHIFT = ("move", [((10, 6), (10, 7))])
TOP = ("move", [((10, 10), (10, 11))])
FOOT = ("move", [((10, 0), (11, 0))])
BOTH = ("move", [((10, 4), (11, 4)), ((10, 6), (11, 6))])
BOTH_FAR = ("move", [((10, 4), (25, 4)), ((10, 6), (25, 6))])
DROP = ("remove", (10, 4))


def cases():
    out = []
    fx = bs.fx(NAME)
    n = [0]

    def c(name, *steps):
        out.append(bs.case(name, *steps))

    def plan(name, edit, topology, part=True):
        c(name, link(fx) if part else link_whole(fx), edit_step(edit), write(edit, topology, "plan", part))

    def tmp():
        n[0] += 1
        path = f"$tmp:komsu-{n[0]}.gpkg"
        return path, {"op": "copyFile", "from": fx, "to": path}

    def run(name, edit, topology, show):
        path, copy = tmp()
        rings = after_write(edit, topology)
        c(name, copy, link(path), edit_step(edit), write(edit, topology, "execute"),
          *[read_back(path, ad, rings[ad]) for ad in show])

    # Plans on the file itself.
    plan("strict: köşesi bir metre kayan komşu geçerli kalır, izler", B_SHIFT, "strict")
    plan("loose: köşesi bir metre kayan komşu izler", B_SHIFT, "loose")
    plan("strict: izleyince kendini kesecek komşu varsa hiçbir şey yazılmaz", B_BOWTIE, "strict")
    plan("loose: izleyince kendini kesecek komşu olduğu gibi kalır, söylenir", B_BOWTIE, "loose")
    plan("strict: izleyince alanı sıfır olacak üçgen yazmayı durdurur", C_FLAT, "strict")
    plan("loose: alanı sıfır olacak üçgen kalır, söylenir", C_FLAT, "loose")
    plan("strict: önceden geçersiz komşu (papyon) izlemeye engel değil, geçerli üçgenle birlikte izler", TOP, "strict")
    plan("loose: önceden geçersiz komşu da izler", TOP, "loose")
    plan("komşular istenmezse kendini kesecek komşu yazmayı durdurmaz: yalnız alan yazılır", B_BOWTIE, None)
    plan("strict: alt köşe kayınca yalnız onu paylaşan komşu izler", FOOT, "strict")
    plan("strict: iki köşe birlikte kayınca iki komşu da izler", BOTH, "strict")
    plan("strict: iki köşe uzağa kayınca biri geçersiz kalır, hiçbir şey yazılmaz", BOTH_FAR, "strict")
    plan("loose: iki köşe uzağa kayınca geçerli kalan izler, kendini kesen kalır", BOTH_FAR, "loose")
    plan("strict: alanın köşesi silinince başka kenarlarla dönen komşu değişmez", DROP, "strict")
    plan("strict: üçgenin köşesi az kayınca üçgen izler", C_SHIFT, "strict")
    plan("bütün tablo bağlıyken alanın dışında komşu yoktur: kendini kesecek olan da çizimdedir", B_BOWTIE, "strict",
         part=False)
    # Writes on a copy, the rows read back.
    run("strict yazma: komşu yeni köşesiyle dosyada", B_SHIFT, "strict", ["A", "B"])
    run("strict yazma reddedilince dosya değişmez", B_BOWTIE, "strict", ["A", "B"])
    run("loose yazma: alan yazılır, kendini kesecek komşu eski hâliyle", B_BOWTIE, "loose", ["A", "B"])
    run("loose yazma: alanı sıfır olacak üçgen eski hâliyle", C_FLAT, "loose", ["A", "C"])
    run("strict yazma: önceden geçersiz papyon köşeyi izler", TOP, "strict", ["C", "D"])
    run("strict yazma: iki köşe birlikte, iki komşu birlikte", BOTH, "strict", ["B", "C"])
    run("loose yazma: geçerli kalan komşu yazılır, kendini kesen kalır", BOTH_FAR, "loose", ["B", "C"])
    # A second write after one that took the neighbours: nothing is left to write.
    path, copy = tmp()
    c("yazmadan sonra katman tabloyla aynı: ikinci yazmada iş yok", copy, link(path), edit_step(B_SHIFT),
      write(B_SHIFT, "strict", "execute"),
      bs.step({"layer": "import-parsel", "direction": "write", "topology": "strict"},
              bs.done({"added": 0, "modified": 0, "removed": 0, "source": ANY, "revision": "$current"}), op="plan"))
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.sources.sync", "commandVersion": 1,
        "title": "Alanın dışındaki komşuların izlemesi ve tam geçerlilik (ADR 0531, 0585)",
        "note": "Satırlar GDAL'ın OGR'ıyla yazılır; izleyen satırlar ADR 0160'ın kuralıyla, geçerlilik GEOS'un IsValid'iyle "
                "(OGR üzerinden) önce ve sonra. scripts/fixtures/source_neighbours_cases.py yazar; PostGIS'teki "
                "karşılığı kentos-postgis'in neighbours_outside_the_area_follow_in_the_database testi.",
        "setup": bs.SETUP,
        "cases": cases(),
    }


def main():
    if "--files" in sys.argv:
        write_files()
        print(f"yazıldı: {FILE.relative_to(ROOT)}")
        return
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırın")
        print(f"{OUT.name}: {len(build()['cases'])} durum")
        return
    OUT.write_text(text)
    print(f"{OUT.name}: {len(build()['cases'])} durum yazıldı")


if __name__ == "__main__":
    main()
