#!/usr/bin/env python3
"""FlatGeobuf alışverişinin ortak durumları ve bağımsız denetimi (docs/adr/0544), KentOS kodu olmadan.

    <GDAL'lı python> scripts/fixtures/fgb_exchange_cases.py --files   # GDAL'ın FlatGeobuf'larını fixtures/fgb/v1'e yazar
    <GDAL'lı python> scripts/fixtures/fgb_exchange_cases.py           # fixtures/commands/v1/file.import.fgb.json'u yazar
    <GDAL'lı python> scripts/fixtures/fgb_exchange_cases.py --check   # durumlar güncel mi; KentOS'un dosyaları OGR'da

GDAL'lı Python: macOS'ta QGIS 4.2'ninki (PYTHONHOME ve PYTHONPATH QGIS'in Resources'ı), betik kendini onunla yeniden açar.

İçe alma: dosyaları GDAL QGIS 4.2.3'ün GeoPackage'ından yazar (konumsuz kaydı olan tabloda dizinsiz, öbürlerinde paketli
Hilbert R-ağacıyla). Durumların beklenenleri OGR'ın FlatGeobuf sürücüsünün okuduğundan, GeoPackage alma durumlarının
kurallarıyla (gpkg_import_cases.py) kurulur; nesnenin kalıcı kimliği UUIDv5(gpkg:fgb/ad.fgb:sıra), sıra dosyadaki 1'den.
Kesik dosyanın yeri biçimin tanımından (başlığın boyu, dizinin boyu 40 × düğüm sayısı) bulunur.

Dışa aktarma: KentOS'un yazdığı dosyalar (`KENTOS_WRITE_FGB=1 cargo test -p kentos-headless --test headless
flatgeobufs` yazar, fixtures/fgb/v1/out) OGR'la açılır: geometriler kaynağın GeoPackage'ındakilerle aynı, alanlar
türleriyle, sistem EPSG:5256, dizinli dosyada OGR'ın kutu süzgeci (dizinden) bütün nesneleri gezen süzgeçle aynı.
Ayrıca fixtures/fgb/v1/geri'deki dosyalar (KentOS'un okuyup yeniden yazdığı GDAL dosyaları,
`KENTOS_WRITE_FGB=1 cargo test -p kentos-geopackage --lib fgb`) OGR'da GDAL'ın kendi dosyalarıyla aynı okunur.
"""

import hashlib
import importlib.util
import json
import os
import sys
import uuid
from pathlib import Path

try:
    from osgeo import gdal, ogr
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

ogr.UseExceptions()
gdal.UseExceptions()
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/fgb/v1"
OUT = ROOT / "fixtures/commands/v1/file.import.fgb.json"
GPKG = ROOT / "fixtures/geopackage/v1/qgis.gpkg"

spec = importlib.util.spec_from_file_location("gpkg_cases", ROOT / "scripts/fixtures/gpkg_import_cases.py")
G = importlib.util.module_from_spec(spec)
spec.loader.exec_module(G)


def index_size(count, node_size):
    """The packed R-tree's bytes: every level's nodes, 40 bytes each (flatgeobuf.org)."""
    if count == 0 or node_size == 0:
        return 0
    node_size = max(node_size, 2)
    n, nodes = count, count
    while True:
        n = -(-n // node_size)
        nodes += n
        if n <= 1:
            return nodes * 40


def write_files():
    DIR.mkdir(parents=True, exist_ok=True)
    plan = [
        # A table with a record (no position) is written without the index, as GDAL refuses it with one.
        ("parsel.fgb", GPKG, "parsel", {"layerCreationOptions": ["SPATIAL_INDEX=NO"]}),
        ("parsel-dizinli.fgb", GPKG, "parsel", {"spatFilter": [-1e9, -1e9, 1e9, 1e9]}),
        ("yol.fgb", GPKG, "yol", {}),
        ("nokta.fgb", GPKG, "nokta", {}),
        ("egri.fgb", GPKG, "egri", {}),
        ("egri_yol.fgb", GPKG, "egri_yol", {}),
        ("malik.fgb", GPKG, "malik", {"layerCreationOptions": ["SPATIAL_INDEX=NO"]}),
        ("yer.fgb", ROOT / "fixtures/geopackage/v1/wgs84.gpkg", "yer", {}),
        ("bos.fgb", GPKG, "nokta", {"where": "1 = 0"}),
        ("sistemsiz.fgb", ROOT / "fixtures/geopackage/v1/sistemsiz.gpkg", None, {}),
    ]
    for name, src, layer, opts in plan:
        (DIR / name).unlink(missing_ok=True)
        layers = [layer] if layer else None
        gdal.VectorTranslate(str(DIR / name), str(src), format="FlatGeobuf", layers=layers, **opts)
    (DIR / "bozuk.fgb").write_bytes(b"PK\x03\x04 bir zip, FlatGeobuf degil")
    # Cut in its first feature: the header and the index whole, ten bytes of the feature.
    data = (DIR / "parsel-dizinli.fgb").read_bytes()
    header = int.from_bytes(data[8:12], "little")
    ds = ogr.Open(str(DIR / "parsel-dizinli.fgb"))
    start = 12 + header + index_size(ds.GetLayer(0).GetFeatureCount(), 16)
    (DIR / "kesik.fgb").write_bytes(data[:start + 10])
    print(f"FlatGeobuf'lar yazıldı: {DIR}")


def read_fgb(name):
    """A FlatGeobuf as OGR reads it, in the shape the GeoPackage cases' `read` gives."""
    path = DIR / name
    ds = gdal.OpenEx(str(path), gdal.OF_VECTOR)
    lyr = ds.GetLayer(0)
    defn = lyr.GetLayerDefn()
    cols = []
    for j in range(defn.GetFieldCount()):
        fd = defn.GetFieldDefn(j)
        kind = "Boolean" if fd.GetSubType() == ogr.OFSTBoolean else fd.GetTypeName()
        cols.append((fd.GetName(), kind, fd.GetWidth()))
    feats = list(lyr)
    wide = {c[0] for j, c in enumerate(cols) if c[1] in ("Integer", "Integer64")
            and any(f.IsFieldSetAndNotNull(j) and abs(f.GetFieldAsInteger64(j)) > G.MAX_INTEGER for f in feats)}
    # docs/adr/0544: a date-time column whose values are all days is a date.
    days = {cname for j, (cname, kind, _) in enumerate(cols) if kind == "DateTime"
            and any(f.IsFieldSetAndNotNull(j) for f in feats)
            and all(len(f.GetFieldAsISO8601DateTime(j)) == 10 for f in feats if f.IsFieldSetAndNotNull(j))}
    rows = []
    for n, f in enumerate(feats, start=1):
        attrs = {}
        for j, (cname, kind, _) in enumerate(cols):
            if not f.IsFieldSetAndNotNull(j):
                continue
            # An empty text is a value, not a null: FlatGeobuf leaves a null out (CBS-06).
            if kind == "Integer64" or (kind == "Integer" and cname in wide):
                v = f.GetFieldAsInteger64(j)
            elif kind in ("Integer", "Boolean"):
                v = f.GetFieldAsInteger(j)
            elif kind == "Real":
                v = f.GetFieldAsDouble(j)
            elif kind == "DateTime":
                v = f.GetFieldAsISO8601DateTime(j)
            else:
                v = f.GetFieldAsString(j)
            attrs[cname] = G.text(v, "Date" if cname in days else kind, cname in wide)
        kind, geo = G.geometry(f.GetGeometryRef())
        rows.append({"fid": n, "uid": str(uuid.uuid5(G.NAMESPACE, f"gpkg:fgb/{name}:{n}")), "kind": kind,
                     "geometry": geo, "attrs": attrs})
    fields = []
    for cname, kind, width in cols:
        k = {"String": "text", "Integer": "integer", "Integer64": "integer", "Real": "decimal",
             "DateTime": "datetime", "Boolean": "boolean"}[kind]
        if cname in wide:  # docs/adr/0629 §4
            k = "bigint"
        if cname in days:
            k = "date"
        fd = {"name": cname, "kind": k}
        if kind == "String" and width:
            fd["length"] = width
        fields.append(fd)
    losses = []
    srs = lyr.GetSpatialRef()
    srid = int(srs.GetAuthorityCode(None)) if srs is not None and srs.GetAuthorityName(None) == "EPSG" else None
    data = path.read_bytes()
    return {"file": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
            "tables": [{"name": lyr.GetName(), "fields": fields, "rows": rows, "losses": losses, "srid": srid}]}


def imported(name, routes=None, start=2):
    G.read = read_fgb
    output, warnings, fields, ids, _ = G.imported(name, None, routes, start)
    if "group" in output:
        # The group is named after the file; when a layer took that id the next free one.
        used = {m["id"] for m in output["layers"]}
        group, k = G.slug(name.removesuffix(".fgb")), 2
        base = group
        while group in used:
            group = f"{base}-{k}"
            k += 1
        output["group"] = group
    return output, warnings, fields, ids


def fx(name):
    return f"$fixture:fgb/v1/{name}"


def take(title, name, extra=None, routes=None, op="execute"):
    output, warnings, fields, ids = imported(name, routes)
    inp = {"path": fx(name), **(extra or {})}
    if op == "plan":
        return G.case(title, G.step(inp, G.done(output, warnings), G.SAME, op="plan"))
    expect = {"ids": [1, *ids], "fields": fields, "revision": "changed", "canUndo": True}
    return G.case(title, G.step(inp, G.done(output, warnings), expect))


def unreadable(name, why):
    return G.case(f"Okunamayan dosya: {name}", G.step({"path": fx(name)}, G.failed(
        "import_unreadable", f"{fx(name)} FlatGeobuf olarak okunamadı: {why}", "path")))


def cases():
    out = [
        take("GDAL'ın dizinsiz parselleri: delikli ve çok parçalı alanlar, konumsuz kayıt, alanlar türleriyle",
             "parsel.fgb"),
        take("GDAL'ın dizinli parselleri: dizin atlanır", "parsel-dizinli.fgb"),
        take("Yaylı, kotlu ve ölçülü yol: CircularString ve CompoundCurve kesin bulge'lara, Z ve M", "yol.fgb"),
        take("Kotlu nokta", "nokta.fgb"),
        take("Eğri alan (CurvePolygon)", "egri.fgb"),
        take("Bileşik eğri (CompoundCurve)", "egri_yol.fgb"),
        take("Geometrisiz tablo: her nesne kayıt", "malik.fgb"),
        take("Plan dosyayı okur, hiçbir şey yazmaz", "parsel.fgb", op="plan"),
        take("Nesneler var olan katmana", "yol.fgb", {"layer": "cizim"}, routes={"yol": "cizim"}),
    ]
    first = read_fgb("yol.fgb")["tables"][0]["rows"][0]["uid"]
    w = read_fgb("yer.fgb")
    out += [
        G.case("Validate yalnız denetler", G.step({"path": fx("parsel.fgb")}, G.done(None), op="validate")),
        G.case("Aynı dosya ikinci kez alınmaz",
               take("x", "yol.fgb")["steps"][0],
               G.step({"path": fx("yol.fgb")},
                      G.failed("entity_exists",
                               f"Dosyadaki bir kaydın kalıcı kimliği ({first}) çizimde zaten bir nesnenin: yol.fgb daha "
                               "önce alınmış. Önceki alımı geri alın ya da başka bir çizimde açın.", "path"),
                      {"ids": [1, 2, 3], "revision": "same"})),
        G.case("WGS 84'teki dosya çevirme istenmeden alınmaz",
               G.step({"path": fx("yer.fgb")},
                      G.failed("crs_mismatch", "yer.fgb EPSG:4326 sisteminde, proje EPSG:5256. Koordinatların "
                                               "projenin sistemine çevrilmesi için reproject verin ya da projeyi "
                                               "EPSG:4326'de açın.", "reproject"))),
        G.case("WGS 84'teki dosya reproject ile çevrilir",
               G.step({"path": fx("yer.fgb"), "reproject": True},
                      G.done({"file": "yer.fgb", "bytes": w["bytes"], "sha256": w["sha256"], "srid": 4326,
                              "moved": True, "objects": 1,
                              "tables": [{"table": "yer", "layer": "import-yer", "read": 1, "objects": 1,
                                          "records": 0, "skipped": 0}],
                              "layers": [{"id": "import-yer", "name": "yer"}], "group": "import-yer-2",
                              "revision": "$current"}),
                      {"ids": [1, 2], "revision": "changed", "canUndo": True,
                       "fields": {"2": {"kind": "point", "layerId": "import-yer", "attrs": {"ad": "Kızılay"}}}})),
        take("Sistem bildirmeyen dosya olduğu gibi alınır (crs_undeclared)", "sistemsiz.fgb"),
        G.case("Nesnesi olmayan dosya",
               G.step({"path": fx("bos.fgb")},
                      G.failed("nothing_to_import", "bos.fgb dosyasının seçilen tablolarında alınacak kayıt yok.",
                               "path"))),
        G.case("Hedef katman kilitli",
               G.step({"path": fx("yol.fgb"), "layer": "kilitli"},
                      G.failed("layer_locked", "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın ya da "
                                               "başka bir katman verin.", "layers.yol"))),
        G.case("Olmayan dosya", G.step({"path": fx("yok.fgb")}, G.failed(
            "file_not_found",
            "$fixture:fgb/v1/yok.fgb okunamadı: No such file or directory (os error 2). Dosyanın yolunu denetleyin.",
            "path"))),
        G.case("Göreli yol", G.step({"path": "parsel.fgb"}, G.failed(
            "invalid_file_path",
            "“parsel.fgb” bir dosyanın mutlak yolu değil; okunacak FlatGeobuf'un tam yolunu verin (/klasör/ad.fgb).",
            "path"))),
        unreadable("bozuk.fgb", "FlatGeobuf dosyası değil (başı “fgb\\x03fgb” değil)"),
        unreadable("kesik.fgb", "nesne dosyanın dışına taşıyor"),
        G.case("Olmayan katman", G.step({"path": fx("yol.fgb"), "layer": "yok"}, G.failed(
            "layer_not_found",
            "“yok” kimlikli katman çizimde yok; “yol” tablosunun kayıtlarını var olan bir katmanın kimliğine yönlendirin.",
            "layers.yol"))),
        G.case("Grup", G.step({"path": fx("yol.fgb"), "layer": "grup"}, G.failed(
            "not_a_layer", "“Grup” bir grup; kayıtlar yalnız katmana konur. Grubun altındaki bir katmanı verin.",
            "layers.yol"))),
    ]
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "file.import.fgb", "commandVersion": 1,
        "title": "FlatGeobuf içe aktar: dizinli ve dizinsiz, yaylar, Z ve M, sistem",
        "note": "ADR 0544. Dosyaları GDAL QGIS 4.2.3'ün GeoPackage'ından yazar; beklenenler OGR'ın FlatGeobuf "
                "sürücüsünün okuduğundan GeoPackage alma durumlarının kuralıyla (gpkg_import_cases.py) kurulur. "
                "Bağımsız başvuru: scripts/fixtures/fgb_exchange_cases.py; elle düzenlemeyin.",
        "setup": G.SETUP,
        "cases": cases(),
    }


def same_geometry(w, g):
    if w is None or g is None:
        return (w is None) == (g is None)
    # A file of mixed layers is Z and M throughout; a coordinate without them is NaN there (none when read back).
    if not w.Is3D() or not w.IsMeasured():
        g = g.Clone()
        if not w.Is3D():
            g.Set3D(False)
        if not w.IsMeasured():
            g.SetMeasured(False)
    try:
        if w.Equals(g):
            return True
        lw, lg = w.GetLinearGeometry(0.0001), g.GetLinearGeometry(0.0001)
        return lw.Within(lg.Buffer(1e-9)) and lg.Within(lw.Buffer(1e-9))
    except RuntimeError:
        return False


def unmatched(want, got):
    """The geometries of `want` no geometry of `got` equals, each matched once (a one-part MULTIPOLYGON equals its
    POLYGON)."""
    left = list(got)
    missing = []
    for w in want:
        k = next((i for i, g in enumerate(left) if same_geometry(w, g)), None)
        if k is None:
            missing.append(w)
        else:
            left.pop(k)
    return missing, left


def check_out():
    """KentOS's files read by OGR: the source's geometries, the fields' types, the system, the index."""
    out = DIR / "out"
    problems = []
    # Comparing a ring whose Z is NaN warns that it is not closed; the comparison says what differs.
    gdal.PushErrorHandler("CPLQuietErrorHandler")
    src = ogr.Open(str(GPKG))

    def geom(f):
        g = f.GetGeometryRef()
        return None if g is None or g.IsEmpty() else g.Clone()

    layers = ["parsel", "yol", "nokta", "egri", "egri_yol"]
    for name in [*layers, "hepsi"]:
        path = out / f"kentos-{name}.fgb"
        if not path.exists():
            problems.append(f"{path.name} yok: önce KENTOS_WRITE_FGB=1 ile KentOS'un testini çalıştırın")
            continue
        ds = ogr.Open(str(path))
        lyr = ds.GetLayer(0)
        sources = layers + ["malik"] if name == "hepsi" else [name]
        want = [geom(f) for s in sources for f in src.GetLayerByName(s)]
        got = [geom(f) for f in lyr]
        missing, extra = unmatched(want, got)
        for w in missing:
            problems.append(f"{path.name}: kaynağın geometrisi yok: {w.ExportToIsoWkt()[:80] if w else None}")
        for g in extra:
            problems.append(f"{path.name}: fazla geometri: {g.ExportToIsoWkt()[:80] if g else None}")
        srs = lyr.GetSpatialRef()
        if srs is None or srs.GetAuthorityCode(None) != "5256":
            problems.append(f"{path.name}: sistem EPSG:5256 değil")
        defn = lyr.GetLayerDefn()
        types = {defn.GetFieldDefn(j).GetName(): defn.GetFieldDefn(j).GetTypeName() for j in range(defn.GetFieldCount())}
        if name == "parsel" and (types.get("alan") != "Real" or types.get("tescil") != "DateTime"
                                 or types.get("kimlik") not in ("Integer64", "String")):
            problems.append(f"{path.name}: alanların türleri OGR'da {types}")
        if name == "hepsi" and "katman" not in types:
            problems.append(f"{path.name}: katman sütunu yok")
        # The index: OGR's box filter (from the index) finds what a walk over every feature finds.
        if lyr.TestCapability(ogr.OLCFastSpatialFilter):
            box = lyr.GetExtent()
            mx, my = (box[0] + box[1]) / 2, (box[2] + box[3]) / 2
            lyr.SetSpatialFilterRect(box[0], box[2], mx, my)
            filtered = sorted(f.GetFID() for f in lyr)
            lyr.SetSpatialFilter(None)
            walked = sorted(f.GetFID() for f in lyr
                            if f.GetGeometryRef() is not None
                            and f.GetGeometryRef().GetEnvelope()[0] <= mx and f.GetGeometryRef().GetEnvelope()[2] <= my)
            if filtered != walked:
                problems.append(f"{path.name}: dizinden süzülen {filtered}, gezilen {walked}")
        elif name not in ("parsel", "hepsi"):
            problems.append(f"{path.name}: dizinsiz (her nesnesi konumlu)")
    # The GDAL files KentOS read and wrote again read in OGR as GDAL's own.
    back = DIR / "geri"
    for p in sorted(DIR.glob("*.fgb")):
        if p.name in ("bozuk.fgb", "kesik.fgb", "bos.fgb", "sistemsiz.fgb"):
            continue
        again = back / p.name
        if not again.exists():
            problems.append(f"geri/{p.name} yok: KENTOS_WRITE_FGB=1 cargo test -p kentos-geopackage --lib fgb")
            continue

        def dump(path):
            ds = ogr.Open(str(path))
            lyr = ds.GetLayer(0)
            rows = sorted((f.GetGeometryRef().ExportToIsoWkt() if f.GetGeometryRef() else "",
                           [f.GetField(i) for i in range(f.GetFieldCount())]) for f in lyr)
            return (lyr.GetName(), lyr.GetGeomType(), [(x.GetName(), x.GetTypeName()) for x in lyr.schema], str(rows))

        if dump(p) != dump(again):
            problems.append(f"geri/{p.name}: OGR'da GDAL'ın dosyasından farklı")
    return problems


def main():
    if "--files" in sys.argv:
        write_files()
        return
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        problems = []
        if OUT.read_text(encoding="utf-8") != text:
            problems.append(f"{OUT.relative_to(ROOT)} güncel değil")
        problems += check_out()
        if problems:
            sys.exit("\n".join(problems))
        print(f"{len(data['cases'])} durum güncel; KentOS'un FlatGeobuf'ları OGR'da kaynağın geometrileriyle aynı.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(data['cases'])} durum yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
