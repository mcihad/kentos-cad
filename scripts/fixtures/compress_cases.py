#!/usr/bin/env python3
"""Parquet sayfalarının sıkıştırmalarının ortak durumları (docs/adr/0628 §1), KentOS kodu olmadan.

    uv run --no-project --with pyarrow --with zstandard --with lz4 python3 scripts/fixtures/compress_cases.py --files
    uv run --no-project --with pyarrow --with zstandard --with lz4 python3 scripts/fixtures/compress_cases.py --check
    uv run --no-project --with pyarrow --with zstandard --with lz4 python3 scripts/fixtures/compress_cases.py --written

Girdiler tohumlu üreteçlerden gelir (Türkçe metin, Parquet sayfası gibi sayılar, WKB, uzun tekrarlar, rastgele baytlar,
boş ve tek bayt). Dosyaları başvuru kitaplıkları yazar: Snappy, GZIP, LZ4_RAW ve Brotli Apache Arrow'un (pyarrow) kendi
codec'leri; ZSTD libzstd 1.5.7 (`zstandard`: düzeyler, sağlama, içerik boyu olmayan çerçeve, akışla bloklar, birden çok
çerçeve, atlanacak çerçeve, uzun eşleşme, sözlük); LZ4 çerçevesi `lz4.frame`, Hadoop'un çerçevesi LZ4 bloklarının önüne
büyük uçlu iki boy yazılarak, çıplak blok `lz4.block`. Her durum açılanın boyunu ve CRC-32'sini söyler; bozuk dosyalar
(yarım, sağlaması bozuk, sözlüklü, uzaklığı dışarıda) reddedilmelidir.

--check dosyaları yeniden yazmaz (kitaplığın sürümü baytları değiştirebilir): her dosyayı başvuru kitaplığıyla açar ve
üreteçlerin girdisiyle karşılaştırır, durumların listesi güncel mi bakar. --written KentOS'un Snappy'yle sıkıştırdıklarını
(`KENTOS_WRITE_COMPRESS=1 cargo test -p kentos-formats --test all compress` yazar, fixtures/compress/v1/out) pyarrow'la açar.
"""

import gzip
import json
import random
import struct
import sys
import zlib
from pathlib import Path

import lz4.block
import lz4.frame
import pyarrow as pa
import zstandard

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/compress/v1"
OUT = DIR / "out"
CASES = DIR / "cases.json"

WORDS = ("parsel ada yol imar sınır köşe kadastro tapu malik hisse yüzölçümü pafta mahalle ilçe il cadde sokak bina "
         "kat bağımsız bölüm kot eğim dere köprü kavşak refüj kaldırım yeşil alan park Çankaya Ömerli Iğdır Şişli "
         "Göğüş Üsküdar çığ ağaç değirmen ölçü nokta poligon aplikasyon").split()


def text(n, seed):
    r = random.Random(seed)
    parts, size = [], 0
    while size < n:
        words = [r.choice(WORDS) for _ in range(r.randint(3, 12))]
        s = " ".join(words).capitalize() + (". " if r.random() < 0.8 else ".\n")
        if r.random() < 0.15:
            s += f"{r.randint(100, 9999)}/{r.randint(1, 99)} parsel {r.randint(1, 500)} m². "
        parts.append(s)
        size += len(s.encode())
    return "".join(parts).encode()[:n]


def doubles(n, seed):
    """A PLAIN page of parcel corner coordinates: eastings and northings with millimetre noise."""
    r = random.Random(seed)
    out = bytearray()
    for i in range(n // 16):
        out += struct.pack("<dd", 500000.0 + (i % 50) * 20.0 + r.random() * 0.001,
                           4400000.0 + (i // 50) * 20.0 + r.random() * 0.001)
    return bytes(out)


def wkb(n, seed):
    """Little-endian WKB rectangles, one after another."""
    r = random.Random(seed)
    out = bytearray()
    while len(out) < n:
        x, y = 500000 + r.randint(0, 9999) * 0.5, 4400000 + r.randint(0, 9999) * 0.5
        w, h = r.randint(10, 40), r.randint(10, 40)
        ring = [(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)]
        out += b"\x01" + struct.pack("<III", 3, 1, len(ring))
        for px, py in ring:
            out += struct.pack("<dd", px, py)
    return bytes(out[:n])


def runs(n, seed):
    """Long runs: a single byte, then runs of random lengths and bytes (RLE blocks, long match lengths)."""
    r = random.Random(seed)
    out = bytearray(b"a" * (n // 2))
    while len(out) < n:
        out += bytes([r.randint(0, 255)]) * r.randint(1, 3000)
    return bytes(out[:n])


def levels(n, seed):
    """Bit-packed definition levels and small dictionary indices, like an RLE/bit-packing hybrid page."""
    r = random.Random(seed)
    out = bytearray()
    while len(out) < n:
        out.append(r.choice((0x03, 0x05, 0xff, 0x00)) if r.random() < 0.3 else r.randint(0, 15))
    return bytes(out[:n])


def noise(n, seed):
    return random.Random(seed).randbytes(n)


INPUTS = {
    "metin": lambda: text(140_000, 1),
    "koordinat": lambda: doubles(40_000, 2),
    "wkb": lambda: wkb(30_000, 3),
    "tekrar": lambda: runs(300_000, 4),
    "duzey": lambda: levels(20_000, 5),
    "rastgele": lambda: noise(20_000, 6),
    "rastgele-buyuk": lambda: noise(135_000, 7),
    "bos": lambda: b"",
    "tek": lambda: b"K",
}


def arrow(codec, data, level=None):
    c = pa.Codec(codec, compression_level=level) if level is not None else pa.Codec(codec)
    return c.compress(data, asbytes=True)


def zstd(data, level=3, **kw):
    return zstandard.ZstdCompressor(level=level, **kw).compress(data)


def zstd_blocks(data, level, step):
    """One frame whose blocks are flushed every `step` bytes: later blocks reuse the tree and the tables before."""
    cobj = zstandard.ZstdCompressor(level=level).compressobj(size=len(data))
    out = bytearray()
    for i in range(0, len(data), step):
        out += cobj.compress(data[i:i + step])
        out += cobj.flush(zstandard.COMPRESSOBJ_FLUSH_BLOCK)
    out += cobj.flush()
    return bytes(out)


def zstd_ldm(data):
    p = zstandard.ZstdCompressionParameters.from_level(19, enable_ldm=True, window_log=22)
    return zstandard.ZstdCompressor(compression_params=p).compress(data)


def zstd_dict(data):
    samples = [text(400, 100 + i) for i in range(400)]
    d = zstandard.train_dictionary(4096, samples)
    return zstandard.ZstdCompressor(level=3, dict_data=d).compress(data)


def skippable(payload):
    return struct.pack("<II", 0x184D2A53, len(payload)) + payload


def hadoop(data, step):
    out = bytearray()
    for i in range(0, max(len(data), 1), step):
        chunk = data[i:i + step]
        block = lz4.block.compress(chunk, store_size=False)
        out += struct.pack(">II", len(chunk), len(block)) + block
    return bytes(out)


def gzip_headers(data):
    """A member with every optional header field: extra, name, comment and the header's CRC-16."""
    head = bytearray(b"\x1f\x8b\x08\x1e\x00\x00\x00\x00\x00\x03")
    head += struct.pack("<H", 6) + b"KT\x02\x00ab"
    head += b"pafta.parquet\x00" + "Çankaya örneği".encode() + b"\x00"
    head += struct.pack("<H", zlib.crc32(bytes(head)) & 0xFFFF)
    c = zlib.compressobj(9, zlib.DEFLATED, -15)
    body = c.compress(data) + c.flush()
    return bytes(head) + body + struct.pack("<II", zlib.crc32(data), len(data) & 0xFFFFFFFF)


def build():
    """(file, codec, input name, bytes, note, error) for every case."""
    I = {k: v() for k, v in INPUTS.items()}
    cases = []

    def add(file, codec, name, data, note, error=None, size=None):
        cases.append(dict(file=file, codec=codec, input=name, bytes=data, note=note, error=error, size=size))

    # SNAPPY: Arrow's codec.
    for name in ("metin", "koordinat", "wkb", "tekrar", "duzey", "rastgele", "bos", "tek"):
        add(f"snappy/{name}.snappy", "SNAPPY", name, arrow("snappy", I[name]), f"Arrow'un Snappy'si, {name}")
    add("snappy/yarim.snappy", "SNAPPY", "wkb", arrow("snappy", I["wkb"])[:-7], "yarım blok", error="yarım")
    # A copy reaching 9 bytes back when 3 are written: varint 12, literal "abc", copy-1 of 4 at offset 9.
    add("snappy/uzak.snappy", "SNAPPY", None, bytes([12, 0x08]) + b"abc" + bytes([0x01, 0x09]),
        "kopyanın uzaklığı yazılanın dışında", error="uzaklık", size=12)

    # GZIP: Arrow's codec at three levels, members, every header field.
    for level in (1, 9):
        add(f"gzip/metin-{level}.gz", "GZIP", "metin", arrow("gzip", I["metin"], level), f"Arrow'un GZIP'i, düzey {level}")
    add("gzip/duzey.gz", "GZIP", "duzey", arrow("gzip", I["duzey"]), "Arrow'un GZIP'i, düzey 6")
    add("gzip/koordinat.gz", "GZIP", "koordinat", arrow("gzip", I["koordinat"]), "Arrow'un GZIP'i, sayılar")
    add("gzip/bos.gz", "GZIP", "bos", arrow("gzip", I["bos"]), "boş girdi")
    half = len(I["metin"]) // 2
    add("gzip/iki-uye.gz", "GZIP", "metin",
        gzip.compress(I["metin"][:half], mtime=0) + gzip.compress(I["metin"][half:], mtime=0), "iki üye arka arkaya")
    add("gzip/basliklar.gz", "GZIP", "wkb", gzip_headers(I["wkb"]), "ek alan, ad, açıklama ve başlığın CRC-16'sı")
    bad = bytearray(arrow("gzip", I["duzey"]))
    bad[-6] ^= 0x40
    add("gzip/saglama.gz", "GZIP", "duzey", bytes(bad), "CRC-32 bozuk", error="sağlama")
    add("gzip/yarim.gz", "GZIP", "duzey", arrow("gzip", I["duzey"])[:-20], "yarım üye", error="yarım")

    # LZ4_RAW: Arrow's codec (one block) and LZ4's high compression.
    for name in ("metin", "koordinat", "tekrar", "rastgele", "bos", "tek"):
        add(f"lz4raw/{name}.lz4", "LZ4_RAW", name, arrow("lz4_raw", I[name]), f"Arrow'un LZ4_RAW'ı, {name}")
    add("lz4raw/metin-yuksek.lz4", "LZ4_RAW", "metin",
        lz4.block.compress(I["metin"], mode="high_compression", compression=12, store_size=False),
        "LZ4'ün yüksek sıkıştırması (HC 12)")
    # A match at offset 0 after the literal "abc".
    add("lz4raw/uzaklik-sifir.lz4", "LZ4_RAW", None, bytes([0x35]) + b"abc" + bytes([0, 0]),
        "eşleşmenin uzaklığı sıfır", error="uzaklık", size=12)
    add("lz4raw/yarim.lz4", "LZ4_RAW", "wkb", arrow("lz4_raw", I["wkb"])[:-9], "yarım blok", error="yarım")

    # LZ4 (Parquet's deprecated codec): Hadoop's framing, an LZ4 frame, a bare block.
    add("lz4/hadoop.lz4", "LZ4", "tekrar", hadoop(I["tekrar"], 65536), "Hadoop'un çerçevesi, 64 KiB'lik parçalar")
    add("lz4/hadoop-sayilar.lz4", "LZ4", "koordinat", hadoop(I["koordinat"], 16384),
        "Hadoop'un çerçevesi, 16 KiB'lik parçalar")
    add("lz4/cerceve.lz4", "LZ4", "wkb", arrow("lz4", I["wkb"]), "Arrow'un LZ4 çerçevesi")
    add("lz4/cerceve-bagli.lz4", "LZ4", "tekrar",
        lz4.frame.compress(I["tekrar"], block_size=lz4.frame.BLOCKSIZE_MAX64KB, block_linked=True,
                           content_checksum=True, block_checksum=True, store_size=True),
        "bağlı bloklu çerçeve, içerik ve blok sağlamalarıyla, boyuyla")
    add("lz4/cerceve-bagimsiz.lz4", "LZ4", "koordinat",
        lz4.frame.compress(I["koordinat"], block_size=lz4.frame.BLOCKSIZE_MAX64KB, block_linked=False,
                           store_size=False),
        "bağımsız bloklu çerçeve, boyu yazılmamış")
    add("lz4/cerceve-ham.lz4", "LZ4", "rastgele", lz4.frame.compress(I["rastgele"]),
        "sıkışmayan veri: çerçevede ham blok")
    add("lz4/ciplak.lz4", "LZ4", "wkb", lz4.block.compress(I["wkb"], store_size=False),
        "çıplak blok (eski parquet-cpp)")

    # ZSTD: libzstd 1.5.7 through `zstandard`.
    for level in (-5, 1, 3, 9, 19, 22):
        add(f"zstd/metin-{level}.zst", "ZSTD", "metin", zstd(I["metin"], level), f"düzey {level}")
    for level in (1, 19):
        add(f"zstd/koordinat-{level}.zst", "ZSTD", "koordinat", zstd(I["koordinat"], level), f"sayılar, düzey {level}")
    add("zstd/wkb.zst", "ZSTD", "wkb", zstd(I["wkb"], 6), "WKB, düzey 6")
    add("zstd/duzey.zst", "ZSTD", "duzey", zstd(I["duzey"], 12), "düzeyler, düzey 12")
    add("zstd/tekrar.zst", "ZSTD", "tekrar", zstd(I["tekrar"], 3), "uzun tekrarlar: RLE blokları")
    add("zstd/rastgele.zst", "ZSTD", "rastgele-buyuk", zstd(I["rastgele-buyuk"], 3), "sıkışmayan veri: ham bloklar")
    add("zstd/bos.zst", "ZSTD", "bos", zstd(I["bos"]), "boş girdi")
    add("zstd/tek.zst", "ZSTD", "tek", zstd(I["tek"]), "tek bayt")
    add("zstd/saglamali.zst", "ZSTD", "metin",
        zstd(I["metin"], 3, write_checksum=True, write_content_size=False), "içerik sağlamalı, boyu yazılmamış")
    add("zstd/bloklar-19.zst", "ZSTD", "metin", zstd_blocks(I["metin"], 19, 8192),
        "8 KiB'de bir blok: önceki ağaç ve tablolar yinelenir")
    add("zstd/bloklar-1.zst", "ZSTD", "koordinat", zstd_blocks(I["koordinat"], 1, 4096), "4 KiB'de bir blok, düzey 1")
    add("zstd/uzun.zst", "ZSTD", "tekrar", zstd_ldm(I["tekrar"]), "uzun eşleşme kipi")
    add("zstd/arrow.zst", "ZSTD", "wkb", arrow("zstd", I["wkb"]), "Arrow'un ZSTD'si")
    add("zstd/iki-cerceve.zst", "ZSTD", "metin", zstd(I["metin"][:half], 3) + zstd(I["metin"][half:], 19),
        "iki çerçeve arka arkaya")
    add("zstd/atlanacak.zst", "ZSTD", "wkb", skippable(b"KentOS") + zstd(I["wkb"], 3) + skippable(b""),
        "önde ve arkada atlanacak çerçeveler")
    one = zstd(I["wkb"], 3, write_checksum=True)
    bad = bytearray(one)
    bad[-1] ^= 0x10
    add("zstd/saglama.zst", "ZSTD", "wkb", bytes(bad), "içerik sağlaması bozuk", error="sağlama")
    add("zstd/yarim.zst", "ZSTD", "wkb", one[:-40], "yarım çerçeve", error="yarım")
    add("zstd/sozluk.zst", "ZSTD", "metin", zstd_dict(I["metin"][:20000]), "sözlüklü çerçeve", error="sözlük",
        size=20000)
    add("zstd/buyuk.zst", "ZSTD", "duzey", zstd(I["duzey"], 3), "boyu büyük beklenen", error="boy",
        size=len(I["duzey"]) + 1)

    # BROTLI: Arrow's codec, refused.
    add("brotli/metin.br", "BROTLI", "metin", arrow("brotli", I["metin"]), "Brotli açılmaz", error="açılmaz")
    return cases, I


def table(cases, I):
    rows = []
    for c in cases:
        data = I[c["input"]] if c["input"] else None
        size = c["size"] if c["size"] is not None else len(data)
        row = {"file": c["file"], "codec": c["codec"], "size": size, "note": c["note"]}
        if c["error"]:
            row["error"] = c["error"]
        else:
            row["crc32"] = zlib.crc32(data)
        rows.append(row)
    return {"version": 1, "inputs": {k: {"size": len(v), "crc32": zlib.crc32(v)} for k, v in I.items()},
            "cases": rows}


def reference_open(codec, raw, size):
    """The bytes a reference library makes of `raw` (independent of KentOS)."""
    if codec in ("SNAPPY", "GZIP", "LZ4_RAW"):
        name = {"SNAPPY": "snappy", "GZIP": "gzip", "LZ4_RAW": "lz4_raw"}[codec]
        if codec == "GZIP" and raw[3] & 0x1e:
            return gzip.decompress(raw)
        return pa.decompress(raw, decompressed_size=size, codec=name, asbytes=True)
    if codec == "ZSTD":
        return zstandard.ZstdDecompressor().decompressobj().decompress(raw) if raw else b""
    if codec == "LZ4":
        if raw[:4] == b"\x04\x22\x4d\x18":
            return lz4.frame.decompress(raw)
        out, i = bytearray(), 0
        try:
            while i < len(raw):
                n, m = struct.unpack(">II", raw[i:i + 8])
                out += lz4.block.decompress(raw[i + 8:i + 8 + m], uncompressed_size=n)
                i += 8 + m
            if len(out) == size:
                return bytes(out)
        except Exception:
            pass
        return lz4.block.decompress(raw, uncompressed_size=size)
    raise ValueError(codec)


def zstd_frames(raw):
    """Every frame of `raw` (zstandard's decompressobj stops at the first)."""
    out, rest = bytearray(), raw
    while rest:
        magic = struct.unpack("<I", rest[:4])[0]
        if magic & 0xFFFFFFF0 == 0x184D2A50:
            rest = rest[8 + struct.unpack("<I", rest[4:8])[0]:]
            continue
        d = zstandard.ZstdDecompressor().decompressobj()
        out += d.decompress(rest)
        rest = d.unused_data
    return bytes(out)


def check():
    cases, I = build()
    want = table(cases, I)
    have = json.loads(CASES.read_text())
    problems = []
    if have != want:
        problems.append("cases.json güncel değil: --files ile yeniden yazın")
    for c in have["cases"]:
        raw = (DIR / c["file"]).read_bytes()
        try:
            if c["codec"] == "ZSTD":
                got = zstd_frames(raw)
            elif c["codec"] == "BROTLI":
                got = pa.decompress(raw, decompressed_size=c["size"], codec="brotli", asbytes=True)
            else:
                got = reference_open(c["codec"], raw, c["size"])
            ok = len(got) == c["size"] and zlib.crc32(got) == c.get("crc32", zlib.crc32(got))
        except Exception as e:  # noqa: BLE001 - every reference error is a refusal here
            got, ok = None, False
            refused = str(e)
        else:
            refused = None
        if "error" in c and c["codec"] != "BROTLI":
            if ok and c["error"] != "boy":
                problems.append(f"{c['file']}: başvuru kitaplığı bozuk dosyayı açtı")
        elif not ok:
            problems.append(f"{c['file']}: başvuru kitaplığı açamadı ({refused})")
    for p in problems:
        print(p)
    print(f"{len(have['cases'])} durum, {len(problems)} sorun")
    return not problems


def written():
    """KentOS's Snappy blocks (fixtures/compress/v1/out) opened by Arrow: each the bytes its source case opens to."""
    index = json.loads((OUT / "index.json").read_text())
    bad = 0
    for row in index:
        raw = (OUT / row["file"]).read_bytes()
        source = (DIR / row["source"]).read_bytes()
        want = zstd_frames(source) if row["codec"] == "ZSTD" else reference_open(row["codec"], source, row["size"])
        got = pa.decompress(raw, decompressed_size=row["size"], codec="snappy", asbytes=True)
        if got != want:
            print(f"{row['file']}: Arrow'un açtığı kaynağın açılanıyla aynı değil")
            bad += 1
    print(f"KentOS'un {len(index)} Snappy bloğu Arrow'da açıldı, {bad} sorun")
    return bad == 0


def write():
    cases, I = build()
    for c in cases:
        p = DIR / c["file"]
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(c["bytes"])
    CASES.write_text(json.dumps(table(cases, I), ensure_ascii=False, indent=1) + "\n")
    print(f"{len(cases)} durum yazıldı, {sum(len(c['bytes']) for c in cases)} bayt")


if __name__ == "__main__":
    if "--files" in sys.argv:
        write()
    elif "--written" in sys.argv:
        sys.exit(0 if written() else 1)
    else:
        sys.exit(0 if check() else 1)
