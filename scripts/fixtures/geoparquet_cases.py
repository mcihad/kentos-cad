#!/usr/bin/env python3
"""GeoParquet alışverişinin ortak durumları ve bağımsız denetimi (docs/adr/0628 §2–§4), KentOS kodu olmadan.

    .run/geo-py/bin/python scripts/fixtures/geoparquet_cases.py --files    # GeoParquet'leri fixtures/geoparquet/v1'e yazar
    .run/geo-py/bin/python scripts/fixtures/geoparquet_cases.py            # fixtures/commands/v1/file.import.parquet.json'u yazar
    .run/geo-py/bin/python scripts/fixtures/geoparquet_cases.py --check    # yazılı durumlar güncel mi
    .run/geo-py/bin/python scripts/fixtures/geoparquet_cases.py --written  # KentOS'un yazdığı GeoParquet'ler pyarrow ve geopandas'ta

`.run/geo-py` sistemin Python'u (GDAL 3.12, geopandas 1.1, shapely 2.1, pyproj 3.7) ve pyarrow 26'dır
(`uv venv --system-site-packages --python /usr/bin/python3 .run/geo-py; uv pip install --python .run/geo-py/bin/python pyarrow==26.0.0`).

İçe alma: dosyaları geopandas (WKB ve GeoArrow, kutu sütunuyla) ve pyarrow (her türden sütun, INT96, kentos_uid,
geometrisiz tablo, ikinci geometri sütunu, sistemsiz ve kimliksiz sistem) yazar. Beklenenler dosyanın pyarrow'un
okuduğu tablosundan kurulur: geometri WKB'si OGR'la GeoPackage alma durumlarının kuralıyla (gpkg_import_cases.py:
alanlar, delikler, parçalar), sütunlar ADR 0628 §2'nin tablosuyla (bu betiğin kuralları), kalıcı kimlik kentos_uid
sütunundan ya da UUIDv5(gpkg:parquet/ad.parquet:satır).

Dışa aktarma: KentOS'un yazdıkları (`KENTOS_WRITE_PARQUET=1 cargo test -p kentos-headless --test headless geoparquet`,
fixtures/geoparquet/v1/out) pyarrow ve geopandas'la açılır: geo anahtarı GeoParquet 1.1'in şemasına uyar, PROJJSON'u
pyproj kodun sistemi olarak okur, geometriler shapely'de geçerli, dış halkalar saat yönü tersine, kutu sütunu her
satırın kutusu, sütunlar türleriyle.
"""

import base64
import decimal
import hashlib
import importlib.util
import json
import math
import struct
import sys
import uuid
from pathlib import Path

import geopandas as gpd
import pyarrow as pa
import pyarrow.parquet as pq
import shapely
from osgeo import ogr

ogr.UseExceptions()
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geoparquet/v1"
OUT = ROOT / "fixtures/commands/v1/file.import.parquet.json"
GPKG = ROOT / "fixtures/geopackage/v1/qgis.gpkg"

spec = importlib.util.spec_from_file_location("gpkg_cases", ROOT / "scripts/fixtures/gpkg_import_cases.py")
G = importlib.util.module_from_spec(spec)
spec.loader.exec_module(G)

TM30 = "EPSG:5256"


def wkb_column(geoms):
    return pa.array([None if g is None else shapely.to_wkb(g, flavor="iso") for g in geoms], type=pa.binary())


def geo_meta(primary="geometry", encoding="WKB", types=(), crs="5256", extra=None, others=None):
    """GeoParquet 1.1's `geo` key; `crs` "omit" leaves the key out (OGC:CRS84), None writes null."""
    col = {"encoding": encoding, "geometry_types": list(types)}
    if crs == "omit":
        pass
    elif crs is None:
        col["crs"] = None
    elif isinstance(crs, dict):
        col["crs"] = crs
    else:
        from pyproj import CRS
        col["crs"] = json.loads(CRS.from_user_input(f"EPSG:{crs}").to_json())
    col.update(extra or {})
    cols = {primary: col}
    cols.update(others or {})
    return {"version": "1.1.0", "primary_column": primary, "columns": cols}


def write_arrow(name, table, geo, **opts):
    meta = dict(table.schema.metadata or {})
    if geo is not None:
        meta[b"geo"] = json.dumps(geo).encode()
    pq.write_table(table.replace_schema_metadata(meta), DIR / name, **opts)


def write_files():
    DIR.mkdir(parents=True, exist_ok=True)
    for p in DIR.glob("*.parquet"):
        p.unlink()
    # geopandas from QGIS's GeoPackage: parcels (holes, two parts, a record), roads and points with Z.
    for table, name, opts in [
        ("parsel", "parsel.parquet", {"write_covering_bbox": True}),
        ("parsel", "parsel-geoarrow.parquet", {"geometry_encoding": "geoarrow"}),
        ("nokta", "nokta.parquet", {}),
        ("nokta", "nokta-geoarrow.parquet", {"geometry_encoding": "geoarrow"}),
        ("yol", "yol.parquet", {"compression": "zstd"}),
    ]:
        df = gpd.read_file(GPKG, layer=table, use_arrow=True)
        # GeoParquet carries Z, not M.
        df.geometry = df.geometry.apply(lambda g: None if g is None else shapely.force_3d(g) if shapely.has_z(g)
                                        else shapely.force_2d(g))
        df.to_parquet(DIR / name, schema_version="1.1.0", **opts)
    wgs = gpd.GeoDataFrame({"ad": ["Kızılay", "Ulus"]},
                           geometry=[shapely.Point(32.8541, 39.9208), shapely.Point(32.8547, 39.9419)], crs="EPSG:4326")
    wgs.to_parquet(DIR / "wgs84.parquet", schema_version="1.1.0")
    # pyarrow by hand: every kind of column (docs/adr/0628 §2).
    pts = [shapely.Point(487000 + 10 * k, 4420000 + 5 * k) for k in range(4)]
    t = pa.table({
        "geometry": wkb_column(pts),
        "evet": pa.array([True, False, None, True]),
        "i8": pa.array([1, -2, None, 127], type=pa.int8()),
        "i32": pa.array([100, None, -7, 2147483647], type=pa.int32()),
        "i64": pa.array([1, 9007199254740993, None, -5], type=pa.int64()),
        "u64": pa.array([1, 18446744073709551615, None, 3], type=pa.uint64()),
        "u32": pa.array([4294967295, 0, None, 7], type=pa.uint32()),
        "f32": pa.array([0.1, None, -2.5, 1e3], type=pa.float32()),
        "f64": pa.array([1234.56, 0.000123, None, -1.5]),
        "yarim": pa.array([1.5, -0.25, None, 65504.0], type=pa.float16()),
        "ond_5_2": pa.array([decimal.Decimal("123.45"), decimal.Decimal("-0.05"), None, decimal.Decimal("999.99")],
                            type=pa.decimal128(5, 2)),
        "ond_20_4": pa.array([decimal.Decimal("1234567890123456.7890"), None, decimal.Decimal("-0.0001"),
                              decimal.Decimal("1.0000")], type=pa.decimal128(20, 4)),
        "metin": pa.array(["Çağlayan", "", None, "Ömerli ğüşıöç"]),
        "kimlik": pa.array([uuid.UUID(int=k * 0x1111).bytes if k != 2 else None for k in range(4)], type=pa.uuid()),
        "tarih": pa.array([19000, -1, None, 2932896], type=pa.int32()).cast(pa.date32()),
        "saat": pa.array([45296789000, 0, None, 86399000000], type=pa.int64()).cast(pa.time64("us")),
        "an": pa.array([1700000000123, 0, None, -86400000], type=pa.int64()).cast(pa.timestamp("ms")),
        "an_utc": pa.array([1700000000000000, None, 0, 1], type=pa.int64()).cast(pa.timestamp("us", tz="UTC")),
        "liste": pa.array([[1, 2, None], [], None, [7]], type=pa.list_(pa.int32())),
        "sozcukler": pa.array([["yol", "ada"], None, ["Ç\"ğ"], []], type=pa.list_(pa.string())),
        "bayt": pa.array([b"\x00\x01", None, b"", b"kentos"], type=pa.binary()),
        "yapi": pa.array([{"a": 1}, None, {"a": 2}, {"a": None}], type=pa.struct([("a", pa.int32())])),
        "esleme": pa.array([[("k", 1)], None, [], [("z", 2)]], type=pa.map_(pa.string(), pa.int32())),
    })
    write_arrow("turler.parquet", t, geo_meta(types=["Point"]))
    # INT96 timestamps and v2 pages, GZIP.
    t = pa.table({"geometry": wkb_column(pts[:3]),
                  "an": pa.array([1700000000123456789, None, -1], type=pa.int64()).cast(pa.timestamp("ns"))})
    write_arrow("int96.parquet", t, geo_meta(types=["Point"]), use_deprecated_int96_timestamps=True,
                data_page_version="2.0", compression="gzip")
    # The lasting ids KentOS wrote.
    t = pa.table({"geometry": wkb_column(pts[:2]), "ad": pa.array(["A", "B"]),
                  "kentos_uid": pa.array(["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7a00", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7a01"])})
    write_arrow("kentos-uid.parquet", t, geo_meta(types=["Point"]))
    # No geometry: a plain Parquet of records.
    t = pa.table({"malik": pa.array(["Ayşe Yılmaz", "Mehmet Öz"]), "hisse": pa.array([0.5, 0.25])})
    write_arrow("kayit.parquet", t, None)
    # A second geometry column, WKT text.
    t = pa.table({"geometry": wkb_column(pts[:2]),
                  "merkez": wkb_column([shapely.Point(487000.5, 4420000.25), None])})
    write_arrow("ikinci-geometri.parquet", t, geo_meta(types=["Point"], others={"merkez": {"encoding": "WKB",
                                                                                          "geometry_types": ["Point"]}}))
    # Systems: null (not known), left out (OGC:CRS84), a PROJJSON without an id; spherical edges.
    t = pa.table({"geometry": wkb_column(pts[:2]), "no": pa.array([1, 2])})
    write_arrow("sistemsiz.parquet", t, geo_meta(types=["Point"], crs=None))
    lonlat = pa.table({"geometry": wkb_column([shapely.Point(32.8541, 39.9208)]), "ad": pa.array(["Kızılay"])})
    write_arrow("crs84.parquet", lonlat, geo_meta(types=["Point"], crs="omit"))
    anon = json.loads(__import__("pyproj").CRS.from_user_input(TM30).to_json())
    anon.pop("id", None)
    anon["name"] = "Adsız TM30"
    write_arrow("kimliksiz-crs.parquet", t, geo_meta(types=["Point"], crs=anon))
    write_arrow("kure.parquet", lonlat, geo_meta(types=["Point"], crs="4326", extra={"edges": "spherical"}))
    # No rows; not a Parquet; Brotli.
    write_arrow("bos.parquet", pa.table({"geometry": wkb_column([])}), geo_meta())
    (DIR / "bozuk.parquet").write_bytes(b"PK\x03\x04 bir zip, Parquet degil, PAR1 ile bitmiyor")
    write_arrow("brotli.parquet", t, geo_meta(types=["Point"]), compression="brotli")
    print(f"GeoParquet'ler yazıldı: {DIR}")


# ── The reference reading (docs/adr/0628 §2, §3) ──────────────────────────────────────────────────────────────────

UNITS = {"ms": (1_000, 3), "us": (1_000_000, 6), "ns": (1_000_000_000, 9)}


def civil(days):
    z = days + 719468
    era = z // 146097
    doe = z - era * 146097
    yoe = (doe - doe // 1460 + doe // 36524 - doe // 146096) // 365
    y = yoe + era * 400
    doy = doe - (365 * yoe + yoe // 4 - yoe // 100)
    mp = (5 * doy + 2) // 153
    d = doy - (153 * mp + 2) // 5 + 1
    m = mp + 3 if mp < 10 else mp - 9
    return (y + 1 if m <= 2 else y), m, d


def clock(seconds, frac, digits):
    f = "" if frac == 0 else "." + str(frac).rjust(digits, "0").rstrip("0")
    return f"{seconds // 3600:02}:{seconds // 60 % 60:02}:{seconds % 60:02}{f}"


def stamp(v, unit, utc):
    per, digits = UNITS[unit]
    seconds, frac = divmod(v, per)
    days, sod = divmod(seconds, 86400)
    y, m, d = civil(days)
    return f"{y:04}-{m:02}-{d:02}T{clock(sod, frac, digits)}{'Z' if utc else ''}"


def real_text(v):
    """A double's text as every KentOS import writes it (docs/adr/0502: the shortest text that reads back): its
    shortest digits written out without an exponent, an integral value without “.0”."""
    t = format(decimal.Decimal(repr(float(v))), "f")
    return t[:-2] if t.endswith(".0") else t


def decimal_text(t):
    """A decimal's canonical text (docs/adr/0199 §1): no leading zeros, “-0” is “0”, the fraction as written."""
    sign = t.startswith("-")
    whole, _, frac = t.lstrip("-").partition(".")
    whole = whole.lstrip("0") or "0"
    zero = set(whole + frac) <= {"0"}
    return ("-" if sign and not zero else "") + whole + ("." + frac if frac else "")


def wkt_num(v):
    return str(int(v)) if v == math.trunc(v) and abs(v) < 1e15 else repr(v)


def column_rule(name, typ, logical, values, losses):
    """A column's field kind and each value's text (None for a null), or none when it is not taken."""
    if pa.types.is_boolean(typ):
        return "boolean", ["true" if v else "false" if v is not None else None for v in values]
    if pa.types.is_integer(typ):
        if pa.types.is_unsigned_integer(typ) and any(v is not None and v > 2**63 - 1 for v in values):
            losses.add(f"“{name}” sütunu 2⁶³'ü aşan işaretsiz sayılar tutuyor: metin alanı olarak alındı")
            return "text", [None if v is None else str(v) for v in values]
        # docs/adr/0629 §4: past 2⁵³ a big integer, every digit.
        if any(v is not None and abs(v) > G.MAX_INTEGER for v in values):
            return "bigint", [None if v is None else str(v) for v in values]
        return "integer", [None if v is None else str(v) for v in values]
    if pa.types.is_floating(typ):
        return "decimal", [None if v is None else real_text(v) for v in values]
    if pa.types.is_decimal(typ):
        if typ.precision <= 15:
            return "decimal", [None if v is None else real_text(float(format(v, "f"))) for v in values]
        # docs/adr/0629 §4: wider ones exactly, a decimal field while every value has 30 digits at most.
        texts = [None if v is None else decimal_text(format(v, "f")) for v in values]
        if all(t is None or len(t.lstrip("-").replace(".", "")) <= 30 for t in texts):
            return "decimal", texts
        return "text", [None if v is None else format(v, "f") for v in values]
    if pa.types.is_string(typ) or pa.types.is_large_string(typ):
        return "text", list(values)
    if isinstance(typ, pa.BaseExtensionType) and typ.extension_name == "arrow.uuid":
        return "uuid", [None if v is None else str(v) for v in values]
    if pa.types.is_date32(typ):
        return "date", [None if v is None else "%04d-%02d-%02d" % civil(v) for v in values]
    if pa.types.is_time(typ):
        out, cut = [], False
        for v in values:
            if v is None:
                out.append(None)
                continue
            t = clock(*divmod(v, UNITS[typ.unit][0]), UNITS[typ.unit][1])
            cut |= len(t) > 8
            out.append(t[:8])
        if cut:
            losses.add(f"“{name}” sütununun saniyenin kesirleri alınmadı: saat alanı saniyeyle yazılır")
        return "time", out
    if pa.types.is_timestamp(typ):
        unit, utc, int96 = logical
        stamps = [None if v is None else stamp(v, unit, utc) for v in values]
        # docs/adr/0629 §4: a timestamp adjusted to UTC is a time stamp; so is an INT96 one (UTC's) with a fraction.
        if (utc and not int96) or (int96 and any(t is not None and len(t) > 20 for t in stamps)):
            return "instant", stamps
        out = []
        for t in stamps:
            if t is None:
                out.append(None)
                continue
            if len(t) > 19 and t[19:] != "Z":
                losses.add(f"“{name}” sütununun saat dilimi ya da saniyenin kesri alınmadı: tarih ve saat alanı "
                           "yerel saattir")
            out.append(t[:19])
        return "datetime", out
    if pa.types.is_list(typ) and not pa.types.is_nested(typ.value_type):
        return "text", [None if v is None else json.dumps(v, ensure_ascii=False, separators=(",", ":"))
                        for v in values]
    if pa.types.is_binary(typ) or pa.types.is_fixed_size_binary(typ) or pa.types.is_large_binary(typ):
        # docs/adr/0629 §4: bytes as a binary field, base64; past 1 MiB a value is left out.
        out = []
        for v in values:
            if v is not None and len(v) > 1 << 20:
                losses.add(f"“{name}” sütununun 1 MiB'ı aşan ikili değerleri alınmadı")
                v = None
            out.append(None if v is None else base64.b64encode(v).decode())
        return "binary", out
    losses.add(f"“{name}” sütunu iç içe (yapı, eşleme ya da liste listesi); alınmadı")
    return None


def system_of(crs_value, missing, losses):
    if missing:
        return 4326
    if crs_value is None:
        return None
    ids = [crs_value.get("id")] + list(crs_value.get("ids") or [])
    for i in ids:
        if i and i.get("authority", "").upper() == "EPSG":
            return int(i["code"])
        if i and i.get("authority", "").upper() == "OGC" and str(i.get("code")) in ("CRS84", "CRS84h"):
            return 4326
    losses.add(f"dosyanın sistemi tanınmadı: “{crs_value.get('name', 'adsız sistem')}” sisteminin bir EPSG kodu yok")
    return None


def read_parquet(name):
    """A GeoParquet as pyarrow reads it, in the shape the GeoPackage cases' `read` gives."""
    path = DIR / name
    f = pq.ParquetFile(path)
    table = f.read()
    meta = f.schema_arrow.metadata or {}
    geo = json.loads(meta[b"geo"]) if b"geo" in meta else None
    losses = set()
    primary, srid, skip, others = None, None, set(), set()
    if geo:
        primary = geo["primary_column"]
        col = geo["columns"][primary]
        srid = system_of(col.get("crs"), "crs" not in col, losses)
        if col.get("edges") == "spherical":
            losses.add("geometrinin kenarları küresel (jeodezik); düzlemde düz kenarlar olarak alındı")
        skip.add(primary)
        for c in geo["columns"].values():
            for p in (c.get("covering") or {}).get("bbox", {}).values():
                skip.add(p[0])
        others = {n for n in geo["columns"] if n != primary}
    skip.add("kentos_uid")
    # Geometries: WKB as it is; GeoArrow through geopandas's own decoding.
    if primary is None:
        geoms = [None] * table.num_rows
    elif geo["columns"][primary]["encoding"].upper() == "WKB":
        geoms = [None if b is None else ogr.CreateGeometryFromWkb(b) for b in table.column(primary).to_pylist()]
    else:
        series = gpd.read_parquet(path).geometry
        geoms = [None if g is None else ogr.CreateGeometryFromWkb(shapely.to_wkb(g, flavor="iso")) for g in series]
    fields, columns = [], {}
    for field in table.schema:
        col = field.name
        if col in skip:
            continue
        values = table.column(col).to_pylist() if not pa.types.is_timestamp(field.type) and not pa.types.is_time(
            field.type) else table.column(col).cast(pa.int64()).to_pylist()
        if col in others:
            losses.add(f"“{col}” geometri sütunu WKT metni olarak alındı")
            wkts = []
            for b in values:
                if b is None:
                    wkts.append(None)
                else:
                    g = shapely.from_wkb(b)
                    wkts.append(f"POINT ({wkt_num(g.x)} {wkt_num(g.y)})")
            fields.append({"name": col, "kind": "text"})
            columns[col] = wkts
            continue
        if pa.types.is_date32(field.type):
            values = table.column(col).cast(pa.int32()).to_pylist()
        if pa.types.is_time32(field.type):
            values = table.column(col).cast(pa.int32()).to_pylist()
        logical = None
        if pa.types.is_timestamp(field.type):
            # INT96 is UTC's (docs/adr/0628 §2): its zone mark, like a fraction, is what the field leaves.
            int96 = f.schema.column(table.schema.get_field_index(col)).physical_type == "INT96"
            logical = (field.type.unit, int96 or field.type.tz is not None, int96)
        rule = column_rule(col, field.type, logical, values, losses)
        if rule is None:
            continue
        kind, texts = rule
        fields.append({"name": col, "kind": kind})
        columns[col] = texts
    uids = table.column("kentos_uid").to_pylist() if "kentos_uid" in table.column_names else None
    rows = []
    for k in range(table.num_rows):
        attrs = {n: v[k] for n, v in columns.items() if v[k] is not None}
        kind, g = G.geometry(geoms[k])
        fid = k + 1
        uid = uids[k] if uids else str(uuid.uuid5(G.NAMESPACE, f"gpkg:parquet/{name}:{fid}"))
        rows.append({"fid": fid, "uid": uid, "kind": kind, "geometry": g, "attrs": attrs})
    data = path.read_bytes()
    return {"file": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
            "tables": [{"name": name.removesuffix(".parquet"), "fields": fields, "rows": rows,
                        "losses": sorted(losses), "srid": srid if srid is not None else FORCE.get("srid")}]}


# The input's `srid`: the system of a file that names none.
FORCE = {}


def imported(name, routes=None, start=2, srid=None):
    G.read = read_parquet
    FORCE["srid"] = srid
    output, warnings, fields, ids, r = G.imported(name, None, routes, start)
    FORCE.clear()
    if "group" in output:
        used = {m["id"] for m in output["layers"]}
        group, k = G.slug(name.removesuffix(".parquet")), 2
        base = group
        while group in used:
            group = f"{base}-{k}"
            k += 1
        output["group"] = group
    return output, warnings, fields, ids, r


def fx(name):
    return f"$fixture:geoparquet/v1/{name}"


def take(title, name, extra=None, routes=None, op="execute", uids=False):
    output, warnings, fields, ids, r = imported(name, routes, srid=(extra or {}).get("srid"))
    inp = {"path": fx(name), **(extra or {})}
    if op == "plan":
        return G.case(title, G.step(inp, G.done(output, warnings), G.SAME, op="plan"))
    expect = {"ids": [1, *ids], "fields": fields, "revision": "changed", "canUndo": True}
    first = G.step(inp, G.done(output, warnings), expect)
    if not uids:
        return G.case(title, first)
    # The objects answer to the file's lasting ids: a command naming them finds them (as file.import.gpkg's case does);
    # the matrix is GEOS's own (shapely).
    rows = r["tables"][0]["rows"]
    a, b = rows[0]["uid"], rows[1]["uid"]
    pa_, pb = rows[0]["geometry"]["p"], rows[1]["geometry"]["p"]
    matrix = shapely.relate(shapely.Point(pa_["x"], pa_["y"]), shapely.Point(pb["x"], pb["y"]))
    relate = G.step({"a": [a], "b": [b], "predicate": "disjoint"},
                    G.done({"pairs": [{"a": a, "b": b, "matrix": matrix}], "revision": "$current"}),
                    {"ids": [1, *ids], "revision": "same"}, command="cad.entities.relate")
    return G.case(title, first, relate)


def refused(title, inp, code, message, path):
    return G.case(title, G.step(inp, G.failed(code, message, path)))


def cases():
    turler = read_parquet("turler.parquet")["tables"][0]
    first = read_parquet("nokta.parquet")["tables"][0]["rows"][0]["uid"]
    out = [
        take("geopandas'ın parselleri: WKB, kutu sütunu alınmaz, delikli ve çok parçalı alanlar, kayıt, alanlar türleriyle",
             "parsel.parquet"),
        take("aynı parseller GeoArrow kodlamasıyla (multipolygon)", "parsel-geoarrow.parquet"),
        take("Z'li noktalar (WKB)", "nokta.parquet"),
        take("Z'li noktalar GeoArrow'un point yapısıyla", "nokta-geoarrow.parquet"),
        take("Z'li yollar, ZSTD sıkıştırmasıyla", "yol.parquet"),
        take("her türden sütun: tam sayılar, işaretsizler, ondalıklar, yarım, metin, UUID, tarih, saat, zaman damgası, "
             "liste; ikili, yapı ve eşleme alınmaz", "turler.parquet"),
        take("INT96 zaman damgaları, v2 sayfaları ve GZIP", "int96.parquet"),
        take("KentOS'un yazdığı kentos_uid kalıcı kimlik olur", "kentos-uid.parquet", uids=True),
        take("geometrisiz Parquet: her satır kayıt", "kayit.parquet"),
        take("ikinci geometri sütunu WKT metni olur", "ikinci-geometri.parquet"),
        take("sistemi null: bilinmiyor, olduğu gibi alınır", "sistemsiz.parquet"),
        take("sistemi null ama srid verilmiş: o sistemde sayılır", "sistemsiz.parquet", {"srid": 5256}),
        take("kimliği olmayan PROJJSON tanınmaz ve söylenir", "kimliksiz-crs.parquet"),
        take("yönlendirilen katmana", "nokta.parquet", {"layer": "cizim"}, routes={"nokta": "cizim"}),
        take("plan hiçbir şey yazmaz", "parsel.parquet", op="plan"),
        G.case("validate yalnız denetler", G.step({"path": fx("parsel.parquet")}, G.done(None), op="validate")),
        G.case("aynı dosya ikinci kez alınmaz",
               G.step({"path": fx("nokta.parquet")}, G.done(*imported("nokta.parquet")[:2]),
                      {"ids": [1, *imported("nokta.parquet")[3]], "revision": "changed", "canUndo": True}),
               G.step({"path": fx("nokta.parquet")},
                      G.failed("entity_exists",
                               f"Dosyadaki bir kaydın kalıcı kimliği ({first}) çizimde zaten bir nesnenin: nokta.parquet "
                               "daha önce alınmış. Önceki alımı geri alın ya da başka bir çizimde açın.", "path"),
                      {"ids": [1, *imported("nokta.parquet")[3]], "revision": "same"})),
        refused("WGS 84'teki dosya çevirme istenmeden alınmaz", {"path": fx("wgs84.parquet")}, "crs_mismatch",
                "wgs84.parquet EPSG:4326 sisteminde, proje EPSG:5256. Koordinatların projenin sistemine çevrilmesi için "
                "reproject verin ya da projeyi EPSG:4326'de açın.", "reproject"),
        refused("crs anahtarı olmayan dosya OGC:CRS84 sayılır", {"path": fx("crs84.parquet")}, "crs_mismatch",
                "crs84.parquet EPSG:4326 sisteminde, proje EPSG:5256. Koordinatların projenin sistemine çevrilmesi için "
                "reproject verin ya da projeyi EPSG:4326'de açın.", "reproject"),
        G.case("WGS 84'teki dosya reproject ile çevrilir",
               G.step({"path": fx("wgs84.parquet"), "reproject": True},
                      G.done({**imported("wgs84.parquet")[0], "moved": True}, []),
                      {"ids": [1, 2, 3], "revision": "changed", "canUndo": True,
                       "fields": {"2": {"kind": "point", "layerId": "import-wgs84", "attrs": {"ad": "Kızılay"}}}})),
        G.case("küresel kenarlı dosya: düzlemde alınır ve söylenir",
               G.step({"path": fx("kure.parquet"), "reproject": True},
                      G.done({**imported("kure.parquet")[0], "moved": True}, imported("kure.parquet")[1]),
                      {"ids": [1, 2], "revision": "changed", "canUndo": True})),
        refused("boş dosya: alınacak satır yok", {"path": fx("bos.parquet")}, "nothing_to_import",
                "bos.parquet dosyasının seçilen tablolarında alınacak kayıt yok.", "path"),
        refused("Parquet olmayan dosya", {"path": fx("bozuk.parquet")}, "import_unreadable",
                f"{fx('bozuk.parquet')} GeoParquet olarak okunamadı: Parquet dosyası değil: PAR1 imzası yok", "path"),
        refused("Brotli sıkıştırması açılmaz", {"path": fx("brotli.parquet")}, "import_unreadable",
                f"{fx('brotli.parquet')} GeoParquet olarak okunamadı: BROTLI sıkıştırması açılmaz; dosyayı Snappy, ZSTD, "
                "GZIP ya da LZ4 ile yeniden yazın", "path"),
        refused("yönlendirilen katman yok", {"path": fx("nokta.parquet"), "layer": "yok"}, "layer_not_found",
                "“yok” kimlikli katman çizimde yok; “nokta” tablosunun kayıtlarını var olan bir katmanın kimliğine "
                "yönlendirin.", "layers.nokta"),
        refused("yönlendirilen katman kilitli", {"path": fx("nokta.parquet"), "layer": "kilitli"}, "layer_locked",
                "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka bir katman verin.",
                "layers.nokta"),
        refused("grup katman değil", {"path": fx("nokta.parquet"), "layer": "grup"}, "not_a_layer",
                "“Grup” bir grup; kayıtlar yalnız katmana konur. Grubun altındaki bir katmanı verin.", "layers.nokta"),
        refused("göreli yol", {"path": "veri/parsel.parquet"}, "invalid_file_path",
                "“veri/parsel.parquet” bir dosyanın mutlak yolu değil; okunacak GeoParquet'in tam yolunu verin "
                "(/klasör/ad.parquet).", "path"),
        refused("olmayan dosya", {"path": fx("yok.parquet")}, "file_not_found",
                f"{fx('yok.parquet')} okunamadı: No such file or directory (os error 2). Dosyanın yolunu denetleyin.",
                "path"),
    ]
    assert turler["fields"]
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "file.import.parquet", "commandVersion": 1,
        "title": "GeoParquet içe aktar",
        "note": "ADR 0628. Dosyaları geopandas ve pyarrow yazar; beklenenler dosyanın pyarrow'un okuduğu tablosundan: "
                "geometri OGR'la GeoPackage alma durumlarının kuralıyla (gpkg_import_cases.py), sütunlar ADR 0628 "
                "§2'nin tablosuyla. Bağımsız başvuru: scripts/fixtures/geoparquet_cases.py; elle düzenlemeyin.",
        "setup": G.SETUP,
        "cases": cases(),
    }


def written():
    """KentOS's GeoParquets read by pyarrow, geopandas, shapely and pyproj."""
    from pyproj import CRS
    out = DIR / "out"
    problems = []
    files = sorted(out.glob("*.parquet"))
    for path in files:
        f = pq.ParquetFile(path)
        geo = json.loads(f.schema_arrow.metadata[b"geo"])
        col = geo["columns"][geo["primary_column"]]
        if geo["version"] != "1.1.0" or col["encoding"] != "WKB":
            problems.append(f"{path.name}: geo anahtarı")
        df = gpd.read_parquet(path)
        crs = col.get("crs")
        if crs is not None:
            epsg = CRS.from_json_dict(crs).to_epsg()
            if df.crs is None or df.crs.to_epsg() != epsg:
                problems.append(f"{path.name}: sistem {epsg}, geopandas {df.crs}")
        table = f.read()
        for k, g in enumerate(df.geometry):
            if g is None or g.is_empty:
                continue
            if not g.is_valid:
                problems.append(f"{path.name}: {k}. geometri geçersiz ({shapely.is_valid_reason(g)})")
            for poly in getattr(g, "geoms", [g]):
                if poly.geom_type == "Polygon":
                    if not poly.exterior.is_ccw or any(r.is_ccw for r in poly.interiors):
                        problems.append(f"{path.name}: {k}. alanın halkalarının yönü")
            if "bbox" in table.column_names:
                b = table.column("bbox")[k].as_py()
                if [b["xmin"], b["ymin"], b["xmax"], b["ymax"]] != list(g.bounds):
                    problems.append(f"{path.name}: {k}. satırın kutusu {b} ≠ {g.bounds}")
        types = sorted({g.geom_type + (" Z" if g.has_z else "") for g in df.geometry if g is not None and not g.is_empty})
        if types != sorted(col["geometry_types"]):
            problems.append(f"{path.name}: geometry_types {col['geometry_types']} ≠ {types}")
        print(f"{path.name}: {f.metadata.num_rows} satır, {f.metadata.num_row_groups} grup, {types}, "
              f"sistem {crs and CRS.from_json_dict(crs).to_epsg()}, sütunlar {table.column_names}")
    for p in problems:
        print(p)
    print(f"{len(files)} dosya, {len(problems)} sorun")
    return not problems and files


def main():
    if "--files" in sys.argv:
        write_files()
        return 0
    if "--written" in sys.argv:
        return 0 if written() else 1
    built = build()
    text = json.dumps(built, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            print(f"{OUT} güncel değil: betiği --check'siz çalıştırın")
            return 1
        print(f"{len(built['cases'])} durum güncel")
        return 0
    OUT.write_text(text)
    print(f"{len(built['cases'])} durum yazıldı: {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
