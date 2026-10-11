#!/usr/bin/env python3
"""Independent reference of the photos' EXIF reader (docs/adr/0225 §3; EXIF 2.32, CIPA DC-008).

Writes the photos of fixtures/exif/v1 by hand and fixtures/exif/v1/cases.json with what each must read as, without
KentOS code: a small JPEG (an 8×6 picture Pillow encoded once, its APP0 JFIF first) into which our own APP1 is put after
the SOI: `Exif\\0\\0`, a TIFF header (II or MM), IFD0 (Make, Model, the Exif IFD's and the GPS IFD's pointers), the
Exif IFD (DateTimeOriginal, OffsetTimeOriginal) and the GPS IFD (the latitude's and longitude's references and three
rationals each, the altitude's reference and rational, the image direction's reference and rational, the GPS date and
time). The expected values follow the ADR's rules with Python's `Fraction`, rounded to a float once (`float()` is
correctly rounded); Pillow's `getexif()` reads every photo back (a cross-check of our writing). The expression core's
reader (`kentos_formats::exif`, crates/shared/formats/tests/all/exif.rs) and the web through WASM
(apps/web/src/io/exif.wasm.test.ts) must give exactly the expected values.

The rules (what the reader says):

- latitude and longitude: d + m/60 + s/3600 from the three rationals, south and west negative; none (and the problem
  “no place”) without a GPS IFD or without both references and values; “cannot be read” for a reference other than
  N/S and E/W, a zero denominator or another count or type; “out of bounds” past 90° or 180°;
- altitude: its rational, negative when the reference byte is 1; direction: its rational, magnetic when its
  reference is `M`;
- taken: DateTimeOriginal as ISO 8601 with OffsetTimeOriginal appended when it is ±HH:MM within ±14:00; without
  DateTimeOriginal (or the unknown zeros) the GPS date stamp and the floor of each of the time stamp's rationals,
  then `Z`;
- camera: the model, after the maker when the model does not start with it (in any case); ASCII up to its first NUL
  without the blanks around it;
- a file that is not a JPEG (nor a TIFF header) is “not a JPEG”; a JPEG without an APP1 Exif before its first scan
  “no EXIF”; a broken IFD0 “broken”.

    python3 scripts/fixtures/exif_cases.py           # write the photos and the cases
    python3 scripts/fixtures/exif_cases.py --check   # compare with what is on disk
"""

import base64
import io
import json
import struct
import sys
from fractions import Fraction
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/exif/v1"

NO_JPEG = "Dosya JPEG değil."
NO_EXIF = "Fotoğrafta EXIF bilgisi yok."
BROKEN = "Fotoğrafın EXIF bilgisi bozuk."
NO_GPS = "Fotoğrafta GPS konumu yok."
BAD_GPS = "Fotoğrafın GPS konumu okunamadı."
FAR_GPS = "Fotoğrafın GPS konumu enlem ya da boylam sınırlarının dışında."

TYPES = {"BYTE": 1, "ASCII": 2, "SHORT": 3, "LONG": 4, "RATIONAL": 5}


def raw_of(kind, values, endian):
    if kind == "ASCII":
        data = values if isinstance(values, bytes) else values.encode("utf-8") + b"\0"
        return data, len(data)
    if kind == "RATIONAL":
        return b"".join(struct.pack(endian + "II", a, b) for a, b in values), len(values)
    if kind == "SHORT":
        return b"".join(struct.pack(endian + "H", v) for v in values), len(values)
    if kind == "LONG":
        return b"".join(struct.pack(endian + "I", v) for v in values), len(values)
    return bytes(values), len(values)


def ifd(entries, endian, base):
    """An IFD at `base` (its offset in the TIFF data): its bytes, its values' area after it; entries by tag."""
    count = len(entries)
    data_at = base + 2 + 12 * count + 4
    body, data = struct.pack(endian + "H", count), b""
    for tag, kind, values in sorted(entries, key=lambda e: e[0]):
        raw, n = raw_of(kind, values, endian)
        if len(raw) <= 4:
            value = raw.ljust(4, b"\0")
        else:
            value = struct.pack(endian + "I", data_at + len(data))
            data += raw + (b"\0" if len(raw) % 2 else b"")
        body += struct.pack(endian + "HHI", tag, TYPES[kind], n) + value
    return body + struct.pack(endian + "I", 0) + data


def tiff(ifd0, exif=None, gps=None, little=True, ifd0_offset=None):
    """A TIFF header and its IFDs: IFD0 at 8, then the Exif IFD and the GPS IFD, their pointers filled in."""
    endian = "<" if little else ">"
    head = (b"II*\0" if little else b"MM\0*") + struct.pack(endian + "I", 8 if ifd0_offset is None else ifd0_offset)
    extra = []
    if exif is not None:
        extra.append((0x8769, "LONG", [0]))
    if gps is not None:
        extra.append((0x8825, "LONG", [0]))
    # IFD0's size does not depend on its pointers' values: lay it out once to know where the others go.
    size0 = len(ifd(ifd0 + extra, endian, 8))
    at = 8 + size0
    pointers, tail = [], b""
    for tag, entries in ((0x8769, exif), (0x8825, gps)):
        if entries is None:
            continue
        block = ifd(entries, endian, at)
        pointers.append((tag, "LONG", [at]))
        tail += block
        at += len(block)
    return head + ifd(ifd0 + pointers, endian, 8) + tail


# An 8×6 picture Pillow 12 encoded once (quality 90, its APP0 JFIF first), kept here so that the photos do not change
# with the encoder's version.
BASE_JPEG = base64.b64decode(
    "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAMCAgMCAgMDAwMEAwMEBQgFBQQEBQoHBwYIDAoMDAsKCwsNDhIQDQ4RDgsLEBYQERMU"
    "FRUVDA8XGBYUGBIUFRT/2wBDAQMEBAUEBQkFBQkUDQsNFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQU"
    "FBQUFBQUFBT/wAARCAAGAAgDASIAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUF"
    "BAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVW"
    "V1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi"
    "4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAEC"
    "AxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVm"
    "Z2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq"
    "8vP09fb3+Pn6/9oADAMBAAIRAxEAPwDTooor+cj99P/Z"
)


def jpeg(app1=None):
    """The small JPEG, our APP1 put after its SOI."""
    body = BASE_JPEG
    if app1 is None:
        return body
    segment = b"Exif\0\0" + app1
    return body[:2] + b"\xff\xe1" + struct.pack(">H", len(segment) + 2) + segment + body[2:]


def r(text):
    """A rational from text: '3192/100' or '39'."""
    a, _, b = text.partition("/")
    return (int(a), int(b) if b else 1)


def gps_ifd(lat=None, lon=None, ns="N", ew="E", alt=None, alt_ref=None, direction=None, dir_ref=None, date=None, time=None):
    out = [(0x0000, "BYTE", [2, 3, 0, 0])]
    if lat is not None:
        out += [(0x0001, "ASCII", ns), (0x0002, "RATIONAL", [r(x) for x in lat])]
    if lon is not None:
        out += [(0x0003, "ASCII", ew), (0x0004, "RATIONAL", [r(x) for x in lon])]
    if alt_ref is not None:
        out.append((0x0005, "BYTE", [alt_ref]))
    if alt is not None:
        out.append((0x0006, "RATIONAL", [r(alt)]))
    if time is not None:
        out.append((0x0007, "RATIONAL", [r(x) for x in time]))
    if dir_ref is not None:
        out.append((0x0010, "ASCII", dir_ref))
    if direction is not None:
        out.append((0x0011, "RATIONAL", [r(direction)]))
    if date is not None:
        out.append((0x001D, "ASCII", date))
    return out


def camera_ifd(make=None, model=None):
    out = [(0x0112, "SHORT", [1])]
    if make is not None:
        out.append((0x010F, "ASCII", make))
    if model is not None:
        out.append((0x0110, "ASCII", model))
    return out


def exif_ifd(original=None, offset=None):
    out = [(0x9000, "BYTE", list(b"0232"))]
    if original is not None:
        out.append((0x9003, "ASCII", original))
    if offset is not None:
        out.append((0x9011, "ASCII", offset))
    return out


# ── The expected values, from the rules ─────────────────────────────────


def frac(text):
    a, b = r(text)
    return None if b == 0 else Fraction(a, b)


def degrees(three):
    parts = [frac(x) for x in three]
    if any(p is None for p in parts):
        return None
    return parts[0] + parts[1] / 60 + parts[2] / 3600


def ascii_text(value):
    raw = value if isinstance(value, bytes) else value.encode("utf-8") + b"\0"
    text = raw.split(b"\0", 1)[0].decode("utf-8", errors="replace").strip(" \t\r\n")
    return text or None


def moment(text):
    if text is None or len(text) != 19 or text[4] != ":" or text[7] != ":" or text[10] != " " or text[13] != ":" or text[16] != ":":
        return None
    try:
        y, mo, d, h, mi, s = int(text[0:4]), int(text[5:7]), int(text[8:10]), int(text[11:13]), int(text[14:16]), int(text[17:19])
    except ValueError:
        return None
    if not all(text[k].isdigit() for k in (0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18)):
        return None
    if y < 1 or not 1 <= mo <= 12 or not 1 <= d <= 31 or h > 23 or mi > 59 or s > 59:
        return None
    return f"{text[0:4]}-{text[5:7]}-{text[8:10]}T{text[11:13]}:{text[14:16]}:{text[17:19]}"


def offset_ok(text):
    if text is None or len(text) != 6 or text[0] not in "+-" or text[3] != ":" or not (text[1:3] + text[4:6]).isdigit():
        return False
    h, m = int(text[1:3]), int(text[4:6])
    return m <= 59 and (h < 14 or (h == 14 and m == 0))


def expected(spec):
    """What the reader must say of a photo written from `spec`."""
    if spec.get("not_jpeg"):
        return {"problem": NO_JPEG}
    if spec.get("no_exif"):
        return {"problem": NO_EXIF}
    if spec.get("broken"):
        return {"problem": BROKEN}
    out = {}
    make = ascii_text(spec["make"]) if "make" in spec else None
    model = ascii_text(spec["model"]) if "model" in spec else None
    camera = (model if model.lower().startswith(make.lower()) else f"{make} {model}") if make and model else (model or make)
    taken = None
    if "original" in spec:
        local = moment(ascii_text(spec["original"]))
        zone = ascii_text(spec["offset"]) if "offset" in spec else None
        if local:
            taken = local + zone if offset_ok(zone) else local
    gps = spec.get("gps")
    if gps is None:
        out["problem"] = NO_GPS
    else:
        if taken is None and gps.get("date") and gps.get("time"):
            hms = [frac(x) for x in gps["time"]]
            if all(v is not None for v in hms):
                h, m, s = (int(v // 1) for v in hms)
                if 0 <= h <= 23 and 0 <= m <= 59 and 0 <= s <= 59:
                    t = moment(f"{ascii_text(gps['date'])} {h:02d}:{m:02d}:{s:02d}")
                    taken = t + "Z" if t else None
        if gps.get("alt") is not None:
            v = frac(gps["alt"])
            if v is not None:
                out["altitude"] = float(-v if gps.get("alt_ref") == 1 else v)
        if gps.get("direction") is not None:
            v = frac(gps["direction"])
            if v is not None:
                out["direction"] = float(v)
                if gps.get("dir_ref") == "M":
                    out["magnetic"] = True
        lat, lon = gps.get("lat"), gps.get("lon")
        if lat is None and lon is None:
            out["problem"] = NO_GPS
        elif lat is None or lon is None:
            out["problem"] = BAD_GPS
        else:
            a, b = degrees(lat), degrees(lon)
            ns, ew = gps.get("ns", "N"), gps.get("ew", "E")
            if a is None or b is None or ns not in ("N", "S") or ew not in ("E", "W"):
                out["problem"] = BAD_GPS
            elif a > 90 or b > 180:
                out["problem"] = FAR_GPS
            else:
                out["latitude"] = float(-a if ns == "S" else a)
                out["longitude"] = float(-b if ew == "W" else b)
    if taken:
        out["taken"] = taken
    if camera:
        out["camera"] = camera
    return out


# ── The photos ─────────────────────────────────────────────────────────

ANKARA = {"lat": ["39", "55", "3192/100"], "lon": ["32", "51", "5863/100"], "alt": "1877/2", "alt_ref": 0, "direction": "541/4", "dir_ref": "T",
          "date": "2024:05:17", "time": ["7", "32", "5"]}

PHOTOS = {
    "konum": {"make": "Canon", "model": "Canon EOS R5", "original": "2024:05:17 10:32:05", "offset": "+03:00", "gps": ANKARA},
    "guney-bati": {"little": False, "make": "Motorola", "model": "moto g84", "original": "2023:11:02 08:15:59",
                   "gps": {"lat": ["33", "51", "5531/100"], "ns": "S", "lon": ["151", "12", "3370/100"], "ew": "W", "alt": "12", "alt_ref": 1,
                           "direction": "10", "dir_ref": "M"}},
    "gps-saati": {"make": "Apple", "model": "iPhone 15 Pro",
                  "gps": {**ANKARA, "time": ["23", "59", "1550/100"], "date": "2024:12:31"}},
    "gpssiz": {"make": "NIKON CORPORATION", "model": "NIKON D850", "original": "2022:07:01 12:00:00", "offset": "+15:00"},
    "exifsiz": {"no_exif": True},
    "sifir-payda": {"make": "Canon", "model": "EOS 90D", "gps": {**ANKARA, "lat": ["39", "55", "3192/0"]}},
    "sinir-disi": {"make": "Canon", "model": "EOS 90D", "gps": {**ANKARA, "lat": ["91", "0", "0"]}},
    "yarim-gps": {"gps": {"lat": ANKARA["lat"]}},
    "ondalik-dakika": {"model": "GoPro HERO12", "gps": {"lat": ["41", "1234567/100000", "0"], "lon": ["28", "5876543/100000", "0"]}},
    "bilinmeyen-yon": {"make": "Canon", "model": "EOS 90D", "gps": {**ANKARA, "ns": "X"}},
    "bos-tarih": {"make": b"Apple\0\0\0\0\0\0", "model": b" iPhone 13  \0", "original": "0000:00:00 00:00:00", "gps": {**ANKARA, "date": None, "time": None}},
    "bozuk-ifd": {"broken": True},
}


def photo_bytes(name, spec):
    if spec.get("no_exif"):
        return jpeg()
    if spec.get("broken"):
        return jpeg(tiff(camera_ifd("Canon", "X"), ifd0_offset=4000))
    little = spec.get("little", True)
    ifd0 = camera_ifd(spec.get("make"), spec.get("model"))
    exif = exif_ifd(spec.get("original"), spec.get("offset")) if ("original" in spec or "offset" in spec) else None
    g = spec.get("gps")
    gps = None
    if g is not None:
        gps = gps_ifd(g.get("lat"), g.get("lon"), g.get("ns", "N"), g.get("ew", "E"), g.get("alt"), g.get("alt_ref"), g.get("direction"), g.get("dir_ref"), g.get("date"), g.get("time"))
    return jpeg(tiff(ifd0, exif, gps, little))


def pillow_check(name, data, spec):
    """Pillow reads our EXIF back: the GPS IFD's references and rationals, the Exif IFD's moment, the camera."""
    if spec.get("no_exif") or spec.get("broken"):
        return
    img = Image.open(io.BytesIO(data))
    exif = img.getexif()
    if "model" in spec:
        assert ascii_text(spec["model"]) == (exif.get(0x0110) or "").strip(" \t\r\n\0") or isinstance(spec["model"], bytes), (name, exif.get(0x0110))
    g = spec.get("gps")
    if g is not None:
        gps = exif.get_ifd(0x8825)
        if g.get("lat") is not None and all(r(x)[1] for x in g["lat"]):
            got = gps.get(2)
            assert got is not None and [Fraction(v.numerator, v.denominator) for v in got] == [frac(x) for x in g["lat"]], (name, got)
            assert gps.get(1) == g.get("ns", "N"), (name, gps.get(1))
        if g.get("lon") is not None:
            got = gps.get(4)
            assert [Fraction(v.numerator, v.denominator) for v in got] == [frac(x) for x in g["lon"]], (name, got)
        if g.get("alt") is not None:
            assert Fraction(gps[6].numerator, gps[6].denominator) == frac(g["alt"]), name
    if "original" in spec:
        assert exif.get_ifd(0x8769).get(0x9003) == ascii_text(spec["original"]), name


def build():
    files, cases = {}, []
    for name, spec in PHOTOS.items():
        data = photo_bytes(name, spec)
        pillow_check(name, data, spec)
        files[f"{name}.jpg"] = data
        cases.append({"file": f"{name}.jpg", **expected(spec)})
    # A TIFF header alone (no picture): the reader takes a TIFF's own header too.
    raw = tiff(camera_ifd("Canon", "EOS 90D"), None, gps_ifd(ANKARA["lat"], ANKARA["lon"]))
    files["yalniz-tiff.tif"] = raw
    cases.append({"file": "yalniz-tiff.tif", **expected({"make": "Canon", "model": "EOS 90D", "gps": {"lat": ANKARA["lat"], "lon": ANKARA["lon"]}})})
    # Not a photo at all.
    files["metin.txt"] = "Bu bir fotoğraf değil.\n".encode("utf-8")
    cases.append({"file": "metin.txt", "problem": NO_JPEG})
    return files, {
        "format": "kentos.exif-cases",
        "version": 1,
        "note": "Written by scripts/fixtures/exif_cases.py from docs/adr/0225 §3 alone; floats are the exact values rounded once.",
        "cases": cases,
    }


def main():
    files, doc = build()
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        bad = [n for n, data in files.items() if not (DIR / n).exists() or (DIR / n).read_bytes() != data]
        if (DIR / "cases.json").read_text(encoding="utf-8") != text:
            bad.append("cases.json")
        if bad:
            sys.exit(f"fixtures/exif/v1 güncel değil ({', '.join(bad)}): betiği --check olmadan çalıştırın ve farkı okuyun")
        print(f"EXIF durumları tutarlı: {len(doc['cases'])} dosya, Pillow hepsini aynı okuyor.")
        return
    DIR.mkdir(parents=True, exist_ok=True)
    for n, data in files.items():
        (DIR / n).write_bytes(data)
    (DIR / "cases.json").write_text(text, encoding="utf-8")
    print(f"fixtures/exif/v1 yazıldı: {len(doc['cases'])} dosya")


if __name__ == "__main__":
    main()
