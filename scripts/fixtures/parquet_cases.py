#!/usr/bin/env python3
"""Parquet okuyucusunun ortak durumları (docs/adr/0628 §1–§2), KentOS kodu olmadan.

    uv run --no-project --with pyarrow --with numpy python3 scripts/fixtures/parquet_cases.py --files
    uv run --no-project --with pyarrow --with numpy python3 scripts/fixtures/parquet_cases.py --check
    uv run --no-project --with pyarrow --with numpy python3 scripts/fixtures/parquet_cases.py --written

Dosyaları Apache Arrow (pyarrow 26) yazar: düz türlerin hepsi (tam sayılar işaretli ve işaretsiz, yarım, tek ve çift
duyarlı sayılar, metin, bayt dizisi, sabit boylu bayt dizisi, UUID, tarih, saat, zaman damgası milli, mikro ve nano,
UTC'li ve UTC'siz, INT96, ondalıklar 32 ve 64 bitlik tam sayıda, 3, 9 ve 17 baytlık dizide), iç içe sütunlar (liste,
liste listesi, yapı, eşleme, yapı listesi, boş ve boş olmayan listeler), sıkıştırmalar (yok, Snappy, GZIP, ZSTD,
LZ4_RAW; Brotli reddedilir), sayfa sürümleri (v1, v2), kodlamalar (sözlük, PLAIN, DELTA_BINARY_PACKED,
DELTA_LENGTH_BYTE_ARRAY, DELTA_BYTE_ARRAY, BYTE_STREAM_SPLIT, RLE), çok satır grubu ve küçük sayfalar.

Beklenenler dosyadan değil, yazılan tablonun kendisinden gelir: her sütunun her satırı anlamıyla JSON (tarih ISO,
saat ve zaman damgası tam sayısından bu betiğin kuralıyla, ondalık kesin metin, bayt 0x ile onaltılık, UUID tireli,
eşleme {key, value} listesi). --check dosyaları pyarrow'la açıp beklenenle karşılaştırır ve durumlar güncel mi bakar.
"""

import decimal
import json
import random
import sys
import uuid
from pathlib import Path

import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/parquet/v1"
CASES = DIR / "cases.json"

UNITS = {"ms": (1_000, 3), "us": (1_000_000, 6), "ns": (1_000_000_000, 9)}


def civil(days):
    """The proleptic Gregorian date of a day count from 1970-01-01."""
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


def fraction(frac, digits):
    return "" if frac == 0 else "." + str(frac).rjust(digits, "0").rstrip("0")


def clock(seconds, frac, digits):
    return f"{seconds // 3600:02}:{seconds // 60 % 60:02}:{seconds % 60:02}{fraction(frac, digits)}"


def time_text(v, unit):
    per, digits = UNITS[unit]
    return clock(v // per, v % per, digits)


def timestamp_text(v, unit, utc):
    per, digits = UNITS[unit]
    seconds, frac = divmod(v, per)
    days, sod = divmod(seconds, 86400)
    y, m, d = civil(days)
    return f"{y:04}-{m:02}-{d:02}T{clock(sod, frac, digits)}{'Z' if utc else ''}"


def to_json(arr, typ, int96=False):
    """Every value of a pyarrow array with its meaning, as this fixture's rules write it."""
    if pa.types.is_timestamp(typ):
        ints = arr.cast(pa.int64()).to_pylist()
        unit = "ns" if int96 else typ.unit
        return [None if v is None else timestamp_text(v, unit, int96 or typ.tz is not None) for v in ints]
    if pa.types.is_time(typ):
        ints = arr.cast(pa.int32() if pa.types.is_time32(typ) else pa.int64()).to_pylist()
        return [None if v is None else time_text(v, typ.unit) for v in ints]
    if pa.types.is_date32(typ):
        days = arr.cast(pa.int32()).to_pylist()
        return [None if v is None else "%04d-%02d-%02d" % civil(v) for v in days]
    if isinstance(typ, pa.BaseExtensionType) and typ.extension_name == "arrow.uuid":
        return [None if v is None else str(v) for v in arr.to_pylist()]
    return [plain(v, typ) for v in arr.to_pylist()]


def plain(v, typ):
    if v is None:
        return None
    if pa.types.is_list(typ) or pa.types.is_large_list(typ) or pa.types.is_fixed_size_list(typ):
        return [plain(x, typ.value_type) for x in v]
    if pa.types.is_map(typ):
        return [{"key": plain(k, typ.key_type), "value": plain(x, typ.item_type)} for k, x in v]
    if pa.types.is_struct(typ):
        return {typ.field(i).name: plain(v[typ.field(i).name], typ.field(i).type) for i in range(typ.num_fields)}
    if pa.types.is_date32(typ):
        return v.isoformat()
    if pa.types.is_decimal(typ):
        return format(v, "f")
    if pa.types.is_binary(typ) or pa.types.is_large_binary(typ) or pa.types.is_fixed_size_binary(typ):
        return "0x" + bytes(v).hex()
    if isinstance(v, float):
        return v
    return v


def nulls(r, n, rate=0.15):
    return [r.random() < rate for _ in range(n)]


def flat(n=60, seed=1):
    r = random.Random(seed)
    gone = lambda: nulls(r, n)  # noqa: E731
    words = "parsel ada yol imar sınır köşe Çankaya Ömerli Iğdır Şişli ğüşıöç".split()

    def col(values, typ, mask=None):
        mask = gone() if mask is None else mask
        return pa.array([None if m else v for v, m in zip(values, mask)], type=typ)

    ts = [r.randint(-2_000_000_000_000_000_000, 2_000_000_000_000_000_000) for _ in range(n)]
    return pa.table({
        "evet": col([r.random() < 0.5 for _ in range(n)], pa.bool_()),
        "i8": col([r.randint(-128, 127) for _ in range(n)], pa.int8()),
        "i16": col([r.randint(-32768, 32767) for _ in range(n)], pa.int16()),
        "i32": col([r.randint(-2**31, 2**31 - 1) for _ in range(n)], pa.int32()),
        "i64": col([r.randint(-2**63, 2**63 - 1) for _ in range(n)], pa.int64()),
        "u8": col([r.randint(0, 255) for _ in range(n)], pa.uint8()),
        "u32": col([r.randint(0, 2**32 - 1) for _ in range(n)], pa.uint32()),
        "u64": col([r.randint(0, 2**64 - 1) for _ in range(n)], pa.uint64()),
        "yarim": pa.array(np.array([r.uniform(-1000, 1000) for _ in range(n)], dtype=np.float16)),
        "f32": col([r.uniform(-1e6, 1e6) for _ in range(n)], pa.float32()),
        "f64": col([r.uniform(-1e9, 1e9) for _ in range(n)], pa.float64()),
        "metin": col([" ".join(r.choice(words) for _ in range(r.randint(0, 4))) for _ in range(n)], pa.string()),
        "bayt": col([r.randbytes(r.randint(0, 9)) for _ in range(n)], pa.binary()),
        "sabit": col([r.randbytes(4) for _ in range(n)], pa.binary(4)),
        "uuid": pa.array([None if m else uuid.UUID(int=r.getrandbits(128)).bytes for m in gone()], type=pa.uuid()),
        "tarih": col([r.randint(-700_000, 2_900_000) for _ in range(n)], pa.int32()).cast(pa.date32()),
        "saat_ms": col([r.randint(0, 86_399_999) for _ in range(n)], pa.int32()).cast(pa.time32("ms")),
        "saat_us": col([r.randint(0, 86_399_999_999) for _ in range(n)], pa.int64()).cast(pa.time64("us")),
        "saat_ns": col([r.randint(0, 86_399_999_999_999) for _ in range(n)], pa.int64()).cast(pa.time64("ns")),
        "an_ms": col([t // 10**6 for t in ts], pa.int64()).cast(pa.timestamp("ms")),
        "an_us_utc": col([t // 1000 for t in ts], pa.int64()).cast(pa.timestamp("us", tz="UTC")),
        "an_ns": col([t // 1000 for t in ts], pa.int64()).cast(pa.timestamp("ns")),
        "ond_5_2": col([decimal.Decimal(r.randint(-99999, 99999)).scaleb(-2) for _ in range(n)], pa.decimal128(5, 2)),
        "ond_20_4": col([decimal.Decimal(r.randint(-10**20 + 1, 10**20 - 1)).scaleb(-4) for _ in range(n)],
                        pa.decimal128(20, 4)),
        "ond_40_6": col([decimal.Decimal(r.randint(-10**40 + 1, 10**40 - 1)).scaleb(-6) for _ in range(n)],
                        pa.decimal256(40, 6)),
    })


def nested(n=40, seed=2):
    r = random.Random(seed)

    def maybe(f, rate=0.15):
        return None if r.random() < rate else f()

    ints = [maybe(lambda: [maybe(lambda: r.randint(-50, 50), 0.2) for _ in range(r.randint(0, 4))]) for _ in range(n)]
    words = [maybe(lambda: [r.choice(["yol", "ada", "köşe", ""]) for _ in range(r.randint(0, 3))]) for _ in range(n)]
    deep = [maybe(lambda: [maybe(lambda: [r.randint(0, 9) for _ in range(r.randint(0, 3))], 0.1)
                           for _ in range(r.randint(0, 3))]) for _ in range(n)]
    structs = [maybe(lambda: {"no": maybe(lambda: r.randint(1, 999), 0.2), "ad": r.choice(["A", "B", "Ç"])})
               for _ in range(n)]
    maps = [maybe(lambda: [(f"k{k}", maybe(lambda: r.randint(0, 100), 0.2)) for k in range(r.randint(0, 3))])
            for _ in range(n)]
    coords = [maybe(lambda: [{"x": r.uniform(0, 1000), "y": r.uniform(0, 1000)} for _ in range(r.randint(0, 4))])
              for _ in range(n)]
    return pa.table({
        "sayilar": pa.array(ints, type=pa.list_(pa.int32())),
        "sozcukler": pa.array(words, type=pa.list_(pa.string())),
        "derin": pa.array(deep, type=pa.list_(pa.list_(pa.int64()))),
        "yapi": pa.array(structs, type=pa.struct([("no", pa.int32()), ("ad", pa.string())])),
        "esleme": pa.array(maps, type=pa.map_(pa.string(), pa.int32())),
        "noktalar": pa.array(coords, type=pa.list_(pa.struct([pa.field("x", pa.float64(), nullable=False),
                                                               pa.field("y", pa.float64(), nullable=False)]))),
    })


def encodings(n=300, seed=3):
    r = random.Random(seed)
    prefix = sorted(f"Parsel/{r.randint(0, 40):03}/{r.randint(0, 999)}" for _ in range(n))
    return pa.table({
        "d64": pa.array([r.randint(-2**62, 2**62) if k % 7 else (2**63 - 1 if k % 2 else -2**63) for k in range(n)],
                        type=pa.int64()),
        "d32": pa.array([None if k % 11 == 0 else r.randint(-1000, 1000) + 5 * k for k in range(n)], type=pa.int32()),
        "boy": pa.array([None if k % 13 == 0 else r.choice(["", "a", "kadastro", "çok uzun bir sözcük"]) for k in range(n)]),
        "onek": pa.array(prefix),
        "bss64": pa.array([r.uniform(-1, 1) * 10 ** r.randint(-5, 5) for _ in range(n)], type=pa.float64()),
        "bss32": pa.array([r.uniform(-1, 1) for _ in range(n)], type=pa.float32()),
        "bssi": pa.array([r.randint(-2**31, 2**31 - 1) for _ in range(n)], type=pa.int32()),
        "bssf": pa.array([r.randbytes(4) for _ in range(n)], type=pa.binary(4)),
        "rle": pa.array([None if k % 17 == 0 else (k // 9) % 2 == 0 for k in range(n)]),
    })


ENCODINGS = {"d64": "DELTA_BINARY_PACKED", "d32": "DELTA_BINARY_PACKED", "boy": "DELTA_LENGTH_BYTE_ARRAY",
             "onek": "DELTA_BYTE_ARRAY", "bss64": "BYTE_STREAM_SPLIT", "bss32": "BYTE_STREAM_SPLIT",
             "bssi": "BYTE_STREAM_SPLIT", "bssf": "BYTE_STREAM_SPLIT", "rle": "RLE"}


def many(n=1000, seed=4):
    r = random.Random(seed)
    return pa.table({
        "no": pa.array(range(n), type=pa.int64()),
        "kod": pa.array([r.choice(["A", "B", "C", "D"]) for _ in range(n)]),
        "deger": pa.array([None if k % 5 == 0 else r.random() for k in range(n)]),
    })


def int96(n=20, seed=5):
    r = random.Random(seed)
    return pa.table({"an": pa.array([None if k % 6 == 0 else r.randint(-10**18, 3 * 10**18) for k in range(n)],
                                    type=pa.int64()).cast(pa.timestamp("ns"))})


def decimals_as_ints(n=30, seed=6):
    r = random.Random(seed)
    return pa.table({
        "d9": pa.array([decimal.Decimal(r.randint(-999_999_999, 999_999_999)).scaleb(-3) for _ in range(n)],
                       type=pa.decimal128(9, 3)),
        "d18": pa.array([decimal.Decimal(r.randint(-10**18 + 1, 10**18 - 1)).scaleb(-5) for _ in range(n)],
                        type=pa.decimal128(18, 5)),
    })


def files():
    """(file, table, writer options, note, error)."""
    out = [
        ("turler.parquet", flat(), {}, "düz türlerin hepsi, sözlüklü, Snappy, v1 sayfaları", None),
        ("turler-v2.parquet", flat(), {"data_page_version": "2.0"}, "aynı tablo v2 sayfalarıyla", None),
        ("turler-yok.parquet", flat(), {"compression": "none", "use_dictionary": False}, "sıkıştırmasız, PLAIN", None),
        ("turler-gzip.parquet", flat(), {"compression": "gzip"}, "GZIP", None),
        ("turler-zstd.parquet", flat(), {"compression": "zstd", "compression_level": 9}, "ZSTD düzey 9", None),
        ("turler-lz4.parquet", flat(), {"compression": "lz4"}, "LZ4_RAW", None),
        ("ic-ice.parquet", nested(), {}, "listeler, liste listesi, yapı, eşleme, yapı listesi", None),
        ("ic-ice-v2.parquet", nested(), {"data_page_version": "2.0", "compression": "zstd"},
         "iç içe sütunlar v2 sayfalarıyla, ZSTD", None),
        ("ic-ice-eski.parquet", nested(), {"use_compliant_nested_type": False}, "listenin eski adları (item)", None),
        ("kodlamalar.parquet", encodings(), {"use_dictionary": False, "column_encoding": ENCODINGS,
                                             "data_page_version": "2.0"},
         "delta, bayt akışı ve RLE kodlamaları, v2", None),
        ("kodlamalar-v1.parquet", encodings(),
         {"use_dictionary": False, "column_encoding": {k: v for k, v in ENCODINGS.items() if v != "RLE"}},
         "delta ve bayt akışı kodlamaları, v1", None),
        ("gruplar.parquet", many(), {"row_group_size": 128, "data_page_size": 256, "write_batch_size": 32},
         "çok satır grubu ve küçük sayfalar", None),
        ("int96.parquet", int96(), {"use_deprecated_int96_timestamps": True}, "INT96 zaman damgaları", None),
        ("ondalik-tamsayi.parquet", decimals_as_ints(), {"store_decimal_as_integer": True},
         "ondalıklar INT32 ve INT64 olarak", None),
        ("brotli.parquet", many(50), {"compression": "brotli"}, "Brotli açılmaz", "BROTLI"),
    ]
    return out


def table_json(t, int96=False):
    return {name: to_json(t.column(name).combine_chunks(), t.schema.field(name).type, int96)
            for name in t.column_names}


def build():
    cases = []
    for name, t, opts, note, error in files():
        cases.append(dict(file=name, table=t, opts=opts, note=note, error=error))
    return cases


def broken(good):
    """Broken files made from a good one."""
    return [
        ("yarim.parquet", good[:-30], "yarım dosya: altbilgi kesik"),
        ("imzasiz.parquet", b"PAR0" + good[4:], "PAR1 imzası yok"),
        ("altbilgi-uzun.parquet", good[:-8] + (len(good)).to_bytes(4, "little") + b"PAR1",
         "altbilginin boyu dosyadan büyük"),
        ("sifreli.parquet", good[:-4] + b"PARE", "şifreli altbilgi"),
    ]


def expected(cases):
    out = []
    for c in cases:
        row = {"file": c["file"], "note": c["note"]}
        if c["error"]:
            row["error"] = c["error"]
        else:
            row["rows"] = c["table"].num_rows
            row["columns"] = table_json(c["table"], c["opts"].get("use_deprecated_int96_timestamps", False))
        out.append(row)
    return out


def write():
    DIR.mkdir(parents=True, exist_ok=True)
    cases = build()
    for c in cases:
        pq.write_table(c["table"], DIR / c["file"], **c["opts"])
    rows = expected(cases)
    good = (DIR / "gruplar.parquet").read_bytes()
    for name, data, note in broken(good):
        (DIR / name).write_bytes(data)
        rows.append({"file": name, "note": note, "error": "bozuk"})
    CASES.write_text(json.dumps({"version": 1, "files": rows}, ensure_ascii=False, indent=1) + "\n")
    print(f"{len(rows)} dosya yazıldı")


def check():
    cases = build()
    want = expected(cases)
    good = (DIR / "gruplar.parquet").read_bytes()
    for name, _, note in broken(good):
        want.append({"file": name, "note": note, "error": "bozuk"})
    have = json.loads(CASES.read_text())["files"]
    problems = []
    if have != want:
        problems.append("cases.json güncel değil: --files ile yeniden yazın")
    for c in cases:
        if c["error"]:
            continue
        read = pq.read_table(DIR / c["file"]).cast(c["table"].schema)
        if not read.equals(c["table"]):
            problems.append(f"{c['file']}: pyarrow'un okuduğu yazılan tablo değil")
        f = pq.ParquetFile(DIR / c["file"])
        md = f.metadata
        codecs = {md.row_group(g).column(i).compression for g in range(md.num_row_groups) for i in range(md.num_columns)}
        encs = {e for g in range(md.num_row_groups) for i in range(md.num_columns)
                for e in md.row_group(g).column(i).encodings}
        print(f"{c['file']:28} {md.num_rows:5} satır, {md.num_row_groups} grup, {sorted(codecs)}, {sorted(encs)}")
    for p in problems:
        print(p)
    print(f"{len(have)} dosya, {len(problems)} sorun")
    return not problems


def written():
    """KentOS's file (fixtures/parquet/v1/out/kentos.parquet) read by Arrow: the table the Rust test wrote."""
    path = DIR / "out/kentos.parquet"
    f = pq.ParquetFile(path)
    t = f.read()
    rows = 1500
    present = [k % 7 != 3 for k in range(rows)]
    n = sum(present)
    ks = iter(range(n))
    vals = [next(ks) if p else None for p in present]
    want = {
        "kimlik": [f"{k:08}" for k in range(rows)],
        "ad": [None if v is None else f"parsel {v % 97} ğüşıöç" for v in vals],
        "no": [None if v is None else v * 3 - 1000 for v in vals],
        "alan": [None if v is None else v / 7.0 for v in vals],
        "var": [None if v is None else v % 3 == 0 for v in vals],
        "gun": [None if v is None else "%04d-%02d-%02d" % civil(v) for v in vals],
        "an": [None if v is None else timestamp_text(v * 1_000_001, "us", False) for v in vals],
    }
    problems = []
    for name, w in want.items():
        got = to_json(t.column(name).combine_chunks(), t.schema.field(name).type)
        if got != w:
            problems.append(f"{name}: Arrow başka okudu")
    box = t.column("bbox").to_pylist()
    for k, v in enumerate(vals):
        b = box[k]
        x = None if v is None else 500000.0 + v * 0.25
        if (b is None) != (v is None) or (b and (b["xmin"], b["xmax"]) != (x, x + 1.0)):
            problems.append(f"bbox satır {k}")
            break
    md = f.metadata
    if md.num_row_groups != 2 or md.metadata.get(b"kentos") != b"deneme":
        problems.append("satır grupları ya da anahtar değerler")
    first = 0
    for g in range(md.num_row_groups):
        rg = md.row_group(g)
        for i in range(md.num_columns):
            c = rg.column(i)
            st = c.statistics
            if st is None or not st.has_null_count:
                problems.append(f"{c.path_in_schema}: istatistik yok")
            elif c.path_in_schema in ("no", "alan", "ad"):
                col = t.column(c.path_in_schema).slice(first, rg.num_rows).drop_null().to_pylist()
                if (st.min, st.max) != (min(col), max(col)) or st.null_count != rg.num_rows - len(col):
                    problems.append(f"{c.path_in_schema}: istatistik {st.min}…{st.max} değerlerle tutmuyor")
        first += rg.num_rows
    print(f"{path.name}: {md.num_rows} satır, {md.num_row_groups} grup, {md.num_columns} sütun, "
          f"an {f.schema_arrow.field('an').type}, gün {f.schema_arrow.field('gun').type}, "
          f"bbox {f.schema_arrow.field('bbox').type}; {len(problems)} sorun")
    for p in problems:
        print(p)
    return not problems


if __name__ == "__main__":
    if "--written" in sys.argv:
        sys.exit(0 if written() else 1)
    elif "--files" in sys.argv:
        write()
    else:
        sys.exit(0 if check() else 1)
