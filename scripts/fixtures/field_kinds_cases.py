#!/usr/bin/env python3
"""Writes the shared cases of the field kinds of docs/adr/0629 §2 (big integer, time stamp, UUID, JSON, binary) through
`cad.entities.set` without KentOS code: a value given to such a field is written in its canonical text, one that does
not keep the kind's rules is `invalid_attribute` with the field's words. The rules and their words are the independent
reference's (scripts/fixtures/layer_field_cases.py).

- fixtures/commands/v1/cad.entities.set.kinds.json

The files KentOS writes of fixtures/kcad/v2/field-kinds.kcad (`KENTOS_WRITE_KINDS=1 cargo test -p kentos-headless
--test headless field_kinds`, fixtures/field-kinds/v1/out) are opened with GDAL's OGR and pyarrow (`--written`): every
column of its type (a big integer Integer64 and INT64, a time stamp DateTime and a UTC TIMESTAMP, a UUID the Parquet UUID,
JSON the Parquet JSON, binary a BLOB and BYTE_ARRAY), every value the same, KentOS's documents of the kinds.

    python3 scripts/fixtures/field_kinds_cases.py           # writes the file
    python3 scripts/fixtures/field_kinds_cases.py --check   # compares it with the one on disk
    .run/geo-py/bin/python scripts/fixtures/field_kinds_cases.py --written
"""
import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from layer_field_cases import check as field_check  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.entities.set.kinds.json"

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
E0, N0 = 487000, 4420000
SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
            "workspace": "gis", "drawingFont": "barlow"}
FIELDS = [
    {"name": "Kimlik", "kind": "bigint"},
    {"name": "Seviye", "kind": "bigint", "min": "-5", "max": "9223372036854775807", "maxExclusive": True},
    {"name": "Ölçüm anı", "alias": "Ölçüldüğü an", "kind": "instant", "min": "2020-01-01T00:00:00Z"},
    {"name": "Cihaz", "kind": "uuid"},
    {"name": "Ek bilgi", "kind": "json"},
    {"name": "İmza", "kind": "binary"},
]
FIELD = {f["name"]: f for f in FIELDS}
START = {"Kimlik": "1", "Cihaz": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001"}


def setup():
    layer = {"id": "olcum", "name": "Ölçüm noktası", "type": "layer", "visible": True, "locked": False, "expanded": True,
             "style": STYLE, "children": [], "fields": copy.deepcopy(FIELDS)}
    point = {"kind": "point", "id": 1, "layerId": "olcum", "attrs": dict(START), "p": {"x": E0 + 10, "y": N0 + 10}}
    return {"format": "kentos.document", "version": 1, "name": "Alan türleri", "settings": SETTINGS,
            "origin": {"x": E0, "y": N0}, "layers": [layer], "activeLayer": "olcum", "entities": [point],
            "styles": {"items": [], "categories": []}}


def written(name, text):
    r = field_check(FIELD[name], text)
    assert "value" in r, (name, text, r)
    return r["value"]


def refused(name, text):
    r = field_check(FIELD[name], text)
    assert "error" in r, (name, text, r)
    return {"status": "failed", "error": {"code": "invalid_attribute", "message": r["message"], "path": f"attrs.{name}"}}


def execute(attrs):
    return {"op": "execute", "input": {"uids": ["$uidOf:1"], "attrs": attrs, "operation": "attributes"}}


def written_case(name, field, text, note=None):
    value = written(field, text)
    after = {**START, field: value}
    if after == START:
        result = {"status": "completed", "output": {"changed": [], "ids": [], "revision": "$current"}, "warnings": []}
        expect = {"ids": [1], "fields": {"1": {"attrs": START}}, "revision": "same"}
    else:
        result = {"status": "completed", "output": {"changed": ["$uidOf:1"], "ids": [1], "revision": "$current"},
                  "warnings": []}
        expect = {"ids": [1], "fields": {"1": {"attrs": after}}, "canUndo": True, "revision": "changed"}
    case = {"name": name, "steps": [{**execute({field: text}), "result": result, "expect": expect}]}
    if note:
        case["note"] = note
    return case


def refused_case(name, field, text, note=None):
    case = {"name": name, "steps": [{**execute({field: text}), "result": refused(field, text),
                                     "expect": {"ids": [1], "fields": {"1": {"attrs": START}}, "revision": "same",
                                                "canUndo": False}}]}
    if note:
        case["note"] = note
    return case


DEEP_64 = "[" * 64 + "]" * 64
DEEP_65 = "[" * 65 + "]" * 65

CASES = [
    written_case("büyük tam sayı: işaret ve baştaki sıfırlar atılır, 2⁵³'ü aşan her basamağıyla", "Kimlik", "+009007199254740993"),
    written_case("büyük tam sayı: en küçük 64 bitlik değer", "Kimlik", "-9223372036854775808"),
    refused_case("büyük tam sayı: 2⁶³ 64 bite sığmaz", "Kimlik", "9223372036854775808"),
    refused_case("büyük tam sayı: kesirli yazılış tam sayı değildir", "Kimlik", "12.0"),
    refused_case("büyük tam sayı: aralığın altı", "Seviye", "-6"),
    refused_case("büyük tam sayı: hariç üst uç", "Seviye", "9223372036854775807"),
    written_case("büyük tam sayı: hariç üst ucun hemen altı", "Seviye", "9223372036854775806"),
    written_case("zaman damgası: kesrin sondaki sıfırları atılır, dilimi iki noktayla", "Ölçüm anı",
                 "2024-03-01T10:20:30.500+0300"),
    written_case("zaman damgası: sıfır dilim Z yazılır, boşlukla ayrılan saat T olur", "Ölçüm anı",
                 "2024-03-01 07:20:30+00:00"),
    written_case("zaman damgası: GG.AA.YYYY gün ve saatin dilimi", "Ölçüm anı", "29.02.2024 23:59:59-05"),
    written_case("zaman damgası: aralığın en azıyla aynı an başka dilimde", "Ölçüm anı", "2020-01-01T03:00:00+03:00",
                 note="UTC'de 2020-01-01T00:00:00: en azdır, alınır (ADR 0629 §2)."),
    refused_case("zaman damgası: aralığın en azından önceki an", "Ölçüm anı", "2020-01-01T02:59:59+03:00"),
    refused_case("zaman damgası: dilimsiz zaman damgası olmaz", "Ölçüm anı", "2024-03-01T10:20:30"),
    refused_case("zaman damgası: dilim en çok ±18:00", "Ölçüm anı", "2024-03-01T10:20:30+18:30"),
    refused_case("zaman damgası: kesir en çok dokuz rakam", "Ölçüm anı", "2024-03-01T10:20:30.1234567891Z"),
    written_case("UUID: süslü ayraçlı büyük harfli küçük harf ve tireli olur", "Cihaz", "{0192F5A0-7C3E-7D4A-9B1E-4C2F8A6D0002}"),
    written_case("UUID: urn:uuid: önekli", "Cihaz", "urn:uuid:0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0003"),
    written_case("UUID: tiresiz 32 rakam tirelenir", "Cihaz", "0192F5A07C3E7D4A9B1E4C2F8A6D0004"),
    written_case("UUID: aynı değerin başka yazılışı değişiklik değildir", "Cihaz", "0192F5A0-7C3E-7D4A-9B1E-4C2F8A6D0001"),
    refused_case("UUID: 31 rakam", "Cihaz", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d001"),
    refused_case("UUID: onaltılık olmayan rakam", "Cihaz", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d000g"),
    written_case("JSON: anlamsız boşluklar atılır, sayılar ve dizgiler yazıldığı gibi kalır", "Ek bilgi",
                 '{ "kot" : [ 1.0e5 , 12345678901234567890 ], "not" : "\\u00e7 ç" }'),
    written_case("JSON: tek bir dizgi ve null da JSON değeridir", "Ek bilgi", ' "metin" '),
    written_case("JSON: 64 düzey iç içe dizi", "Ek bilgi", DEEP_64),
    refused_case("JSON: 65 düzey iç içe dizi", "Ek bilgi", DEEP_65),
    refused_case("JSON: sondaki virgül", "Ek bilgi", '{"a": 1,}'),
    refused_case("JSON: tek tırnaklı dizgi", "Ek bilgi", "{'a': 1}"),
    written_case("ikili: dolgusuz base64 dolgulu yazılır", "İmza", "QUI"),
    written_case("ikili: araya giren boşluklar atlanır", "İmza", "QU JD\nRA=="),
    refused_case("ikili: base64'te olmayan karakter", "İmza", "QU*D"),
    refused_case("ikili: eksik kalan tek karakter", "İmza", "QUJDR"),
]

# A big integer, a time stamp and a UUID written together and undone in one step.
together = {"Kimlik": "  -42 ", "Ölçüm anı": "2024-03-01T10:20:30Z", "Cihaz": "{0192F5A0-7C3E-7D4A-9B1E-4C2F8A6D0005}"}
after = {**START, **{k: written(k, v) for k, v in together.items()}}
CASES.append({
    "name": "birlikte yazılan üç tür tek adımdır, geri alınınca ilk değerler döner; null özniteliği siler",
    "steps": [
        {**execute(together), "result": {"status": "completed", "output": {"changed": ["$uidOf:1"], "ids": [1],
                                                                           "revision": "$current"}, "warnings": []},
         "expect": {"ids": [1], "fields": {"1": {"attrs": after}}, "canUndo": True, "revision": "changed"}},
        {"op": "undo", "returns": "Değiştir", "expect": {"fields": {"1": {"attrs": START}}, "canUndo": False}},
        {**execute({"Cihaz": None}), "result": {"status": "completed", "output": {"changed": ["$uidOf:1"], "ids": [1],
                                                                                  "revision": "$current"}, "warnings": []},
         "expect": {"fields": {"1": {"attrs": {"Kimlik": "1"}}}, "revision": "changed"}},
    ],
})


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.entities.set",
        "commandVersion": 1,
        "title": "Öznitelikler ve alanın yeni türleri",
        "note": "ADR 0629 §2: büyük tam sayı, zaman damgası, UUID, JSON ve ikili alanlara verilen değer tek biçimiyle yazılır, "
                "kurala uymayan invalid_attribute. Written by scripts/fixtures/field_kinds_cases.py without KentOS code; "
                "do not edit by hand.",
        "setup": setup(),
        "cases": CASES,
    }


WRITTEN = ROOT / "fixtures/field-kinds/v1/out"
ROWS = [
    {"Kimlik": 9007199254740993, "Ölçüm anı": (2024, 3, 1, 7, 20, 30, 500000), "Cihaz": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001",
     "Ek bilgi": '{"kot":[1.25,12345678901234567890],"not":"ç"}', "İmza": b"ABC"},
    {"Kimlik": -9223372036854775808, "Ölçüm anı": (2024, 3, 1, 7, 20, 30, 0), "Cihaz": None, "Ek bilgi": "[]", "İmza": None},
]
KINDS = {"Kimlik": "bigint", "Ölçüm anı": "instant", "Cihaz": "uuid", "Ek bilgi": "json", "İmza": "binary"}


def written():
    """KentOS's files of field-kinds.kcad, opened with OGR and pyarrow; the problems found."""
    import datetime
    import sqlite3

    import pyarrow as pa
    import pyarrow.parquet as pq
    from osgeo import gdal, ogr

    ogr.UseExceptions()
    problems = []
    utc = datetime.timezone.utc

    def moment(t):
        return datetime.datetime(*t, tzinfo=utc)

    # OGR: GeoPackage, FlatGeobuf, GML (its names without spaces); a GeoPackage's DATETIME read without a warning.
    want_types = {"Kimlik": "Integer64", "Ölçüm anı": "DateTime", "Cihaz": "String", "Ek bilgi": "String", "İmza": "Binary"}
    for name, rename, binary in (("turler.gpkg", False, True), ("turler.fgb", False, True), ("turler.gml", True, False)):
        gdal.ErrorReset()
        warned = []
        gdal.PushErrorHandler(lambda cls, no, msg: warned.append(msg))
        ds = ogr.Open(str(WRITTEN / name))
        layer = ds.GetLayer(0)
        defn = layer.GetLayerDefn()
        key = (lambda k: k.replace(" ", "_")) if rename else (lambda k: k)
        for k, t in want_types.items():
            i = defn.GetFieldIndex(key(k))
            got = defn.GetFieldDefn(i).GetTypeName() if i >= 0 else None
            if not binary and k == "İmza":
                t = "String"
            if got != t:
                problems.append(f"{name}: “{k}” {got}, beklenen {t}")
        rows = sorted(layer, key=lambda f: f.GetFieldAsInteger64(key("Kimlik")))
        gdal.PopErrorHandler()
        if warned:
            problems.append(f"{name}: GDAL uyardı: {warned}")
        for f, want in zip(rows, sorted(ROWS, key=lambda r: r["Kimlik"])):
            if f.GetFieldAsInteger64(key("Kimlik")) != want["Kimlik"]:
                problems.append(f"{name}: Kimlik {f.GetFieldAsInteger64(key('Kimlik'))}")
            y, mo, d, h, mi, sec, tz = f.GetFieldAsDateTime(key("Ölçüm anı"))
            w = want["Ölçüm anı"]
            if (y, mo, d, h, mi) != w[:5] or abs(sec - (w[5] + w[6] / 1e6)) > 1e-6 or tz != 100:
                problems.append(f"{name}: Ölçüm anı {(y, mo, d, h, mi, sec, tz)}")
            for k in ("Cihaz", "Ek bilgi"):
                got = f.GetFieldAsString(key(k)) if f.IsFieldSetAndNotNull(key(k)) else None
                if got != want[k]:
                    problems.append(f"{name}: {k} {got!r}")
            i = defn.GetFieldIndex(key("İmza"))
            got = (f.GetFieldAsBinary(i) if binary else __import__("base64").b64decode(f.GetFieldAsString(i))) \
                if f.IsFieldSetAndNotNull(i) else None
            if (got or None) != want["İmza"]:
                problems.append(f"{name}: İmza {got!r}")
    con = sqlite3.connect(WRITTEN / "turler.gpkg")
    docs = [json.loads(m) for u, m in con.execute("SELECT md_standard_uri, metadata FROM gpkg_metadata")
            if u == "https://kentos.app/fields/1"]
    if docs != [{"kinds": KINDS}]:
        problems.append(f"turler.gpkg: KentOS belgesi {docs}")
    # pyarrow: the Parquet types and values, the file's `kentos` key.
    f = pq.ParquetFile(WRITTEN / "turler.parquet")
    schema = f.schema
    logical = {schema.column(i).name: (schema.column(i).physical_type, str(schema.column(i).logical_type))
               for i in range(len(schema))}
    want_logical = {"Kimlik": ("INT64", "None"), "Cihaz": ("FIXED_LEN_BYTE_ARRAY", "UUID"),
                    "Ek bilgi": ("BYTE_ARRAY", "JSON"), "İmza": ("BYTE_ARRAY", "None")}
    for k, w in want_logical.items():
        if logical.get(k) != w:
            problems.append(f"turler.parquet: “{k}” {logical.get(k)}, beklenen {w}")
    if not (logical["Ölçüm anı"][0] == "INT64" and "isAdjustedToUTC=true" in logical["Ölçüm anı"][1]
            and "microseconds" in logical["Ölçüm anı"][1]):
        problems.append(f"turler.parquet: Ölçüm anı {logical['Ölçüm anı']}")
    table = f.read()
    got = sorted(table.to_pylist(), key=lambda r: r["Kimlik"])
    for r, want in zip(got, sorted(ROWS, key=lambda r: r["Kimlik"])):
        if r["Kimlik"] != want["Kimlik"] or r["Ölçüm anı"] != moment(want["Ölçüm anı"]) \
                or (str(r["Cihaz"]) if r["Cihaz"] is not None else None) != want["Cihaz"] \
                or r["Ek bilgi"] != want["Ek bilgi"] or r["İmza"] != want["İmza"]:
            problems.append(f"turler.parquet: satır {r}")
    if json.loads(f.metadata.metadata[b"kentos"]) != {"kinds": KINDS}:
        problems.append(f"turler.parquet: kentos {f.metadata.metadata[b'kentos']}")
    if not isinstance(table.schema.field("Ölçüm anı").type, pa.TimestampType):
        problems.append("turler.parquet: Ölçüm anı zaman damgası değil")
    return problems


def main():
    if "--written" in sys.argv[1:]:
        problems = written()
        for p in problems:
            print(p, file=sys.stderr)
        if problems:
            return 1
        print("KentOS'un yazdığı GeoPackage, GeoParquet, FlatGeobuf ve GML OGR'da ve pyarrow'da türleri ve değerleriyle aynı")
        return 0
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"alan türlerinin durumları güncel: {len(CASES)} durum")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(CASES)} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
