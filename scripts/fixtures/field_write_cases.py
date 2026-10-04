#!/usr/bin/env python3
"""Independent reference of the instrument coordinate files (docs/adr/0169 §4; step 7a).

Writes fixtures/field/v1/write.json from the formats' descriptions alone, no KentOS code: points (name, east, north,
elevation, code) written as an instrument takes them in, and every point a format cannot carry said.

The sources are the readers' (docs/adr/0169 §1): Leica's "GSI ONLINE for Leica TPS and DNA" (2003; text data right
justified and zero filled, "PUT/16....+0000A100"), Topcon Link Reference Manual Appendix C ("GTS-7 Points Coordinate
Format": name, easting, northing, elevation), Trimble JobXML 5.3 (FieldBook PointRecord, Grid North, East, Elevation),
Nikon RAW V2.00 ("type, pt, (pt id), northing, easting, elevation, code"; UP an uploaded point).

The rules:

1. Values are metres written with three decimals by the display rule (docs/adr/0149, numeric_display.shown): the
   files carry millimetres. GSI writes them as whole millimetres (units digit 0), right justified and zero filled, the
   sign before; a value with more digits than the word holds (GSI-8: 8, GSI-16: 16) is not carried.
2. Points in their order. A point a format cannot carry is not written and is said, its first problem only, in this
   order: no name; a control character (below U+0020, U+007F) in its name; a blank at its name's start or end (every
   reader trims it); a character the format does not take (GSI: other than U+0021–U+007E; GTS-7 and Nikon RAW: other
   than U+0020–U+007E, and the comma; CSV: the comma; JobXML: none); a name longer than the format takes (GSI-16 16,
   GSI-8 8 characters); a GSI name of more than one character beginning with 0 (GSI drops leading zeros); then the
   same for its code; then a value that is not a number, or that GSI's word does not hold (east, north, elevation).
3. GSI-16: a block per point: “*”, then the words, each followed by a blank: 11 with the block number (the written
   points counted from 1, four digits, modulo 10000) and the name; 81..10 east, 82..10 north, 83..10 elevation (input
   mode 1, keyboard; units 0, metres and millimetres) when there is one; 71.... the code when there is one; CR LF.
   GSI-8: the same without “*”, eight-character data.
4. Topcon GTS-7 points: “name,east,north,elevation,code” per point (an empty field for no elevation or code), CR LF.
5. Trimble JobXML: version 5.3, the job's name and the time stamp; in its FieldBook a PointRecord per point, its ID the
   written points counted from 1 in eight hexadecimal digits: Name, Code (when there is one), Method KeyedIn,
   Classification Normal, Deleted false, Grid with North, East and Elevation (when there is one); text and attributes
   with &, <, >, " escaped; two blanks of indent; CR LF.
6. Nikon RAW: “CO,Nikon RAW data format V2.00”, “CO,” and the job's name, “CO,Dist Units: Metres”, then
   “UP,name,,north,east,elevation,code” per point, CR LF.
7. CSV: “name,east,north,elevation,code” per point (Y, X: east first), UTF-8, CR LF.
8. The job's name is said and nothing written when Nikon RAW cannot carry it (a character other than U+0020–U+007E)
   or it has a control character (JobXML).
"""

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from numeric_display import shown  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "field" / "v1" / "write.json"
NAMES = {"gsi16": "Leica GSI-16", "gsi8": "Leica GSI-8", "gts7": "Topcon GTS-7", "jobxml": "Trimble JobXML", "nikon": "Nikon RAW", "csv": "CSV"}
TEXTS = {
    "empty": "{index}. noktanın adı yok; yazılmadı.",
    "control": "“{name}” adında denetim karakteri var; yazılmadı.",
    "blank": "“{name}” adının başında ya da sonunda boşluk var; okunurken atılır; yazılmadı.",
    "chars": "“{name}” adındaki {chars} {format} dosyasında taşınamaz; yazılmadı.",
    "long": "“{name}” adı {length} karakter; {format} en çok {limit} karakter alır; yazılmadı.",
    "zeros": "“{name}” adı sıfırla başlıyor; GSI baştaki sıfırları atar; yazılmadı.",
    "codeControl": "“{name}” noktasının kodunda denetim karakteri var; yazılmadı.",
    "codeBlank": "“{name}” noktasının kodunun başında ya da sonunda boşluk var; okunurken atılır; yazılmadı.",
    "codeChars": "“{name}” noktasının kodundaki {chars} {format} dosyasında taşınamaz; yazılmadı.",
    "codeLong": "“{name}” noktasının kodu {length} karakter; {format} en çok {limit} karakter alır; yazılmadı.",
    "codeZeros": "“{name}” noktasının kodu sıfırla başlıyor; GSI baştaki sıfırları atar; yazılmadı.",
    "value": "“{name}” noktasının {what} değeri sayı değil; yazılmadı.",
    "range": "“{name}” noktasının {what} değeri {value} m; {format} sözcüğüne sığmıyor; yazılmadı.",
    "job": "İş adı “{job}” {format} dosyasında taşınamaz; dosya yazılmadı.",
}
LIMITS = {"gsi16": 16, "gsi8": 8}
WHAT = {"east": "doğu", "north": "kuzey", "elevation": "kot"}


def control(s):
    return any(ord(c) < 0x20 or ord(c) == 0x7F for c in s)


def foreign(s, fmt):
    """The characters of s the format does not take, in their first order."""
    def bad(c):
        if fmt in ("gsi16", "gsi8"):
            return not (0x21 <= ord(c) <= 0x7E)
        if fmt in ("gts7", "nikon"):
            return not (0x20 <= ord(c) <= 0x7E) or c == ","
        if fmt == "csv":
            return c == ","
        return False

    out = []
    for c in s:
        if bad(c) and c not in out:
            out.append(c)
    return out


def chars_text(cs):
    return ", ".join("boşluk" if c == " " else f"“{c}”" for c in cs)


def text_problem(s, fmt, prefix):
    """The first problem of a name (prefix "") or a code (prefix "code"), or None."""
    key = (lambda k: k if not prefix else prefix + k[0].upper() + k[1:])
    if control(s):
        return key("control"), {}
    if s and (s[0].isspace() or s[-1].isspace()):
        return key("blank"), {}
    cs = foreign(s, fmt)
    if cs:
        return key("chars"), {"chars": chars_text(cs)}
    if fmt in LIMITS and len(s) > LIMITS[fmt]:
        return key("long"), {"length": len(s), "limit": LIMITS[fmt]}
    if fmt in LIMITS and len(s) > 1 and s[0] == "0":
        return key("zeros"), {}
    return None


def mm_digits(v):
    t = shown(v, 3)
    sign = "-" if t.startswith("-") else "+"
    return sign, t.lstrip("-").replace(".", "").lstrip("0") or "0"


def problem(i, p, fmt):
    name, code = p["name"], p.get("code") or ""
    if not name:
        return TEXTS["empty"].format(index=i)
    found = text_problem(name, fmt, "")
    if not found and code:
        found = text_problem(code, fmt, "code")
    if found:
        k, fill = found
        return TEXTS[k].format(name=name, format=NAMES[fmt], **fill)
    for key in ("east", "north", "elevation"):
        v = p.get(key)
        if v is None:
            continue
        if not math.isfinite(v):
            return TEXTS["value"].format(name=name, what=WHAT[key])
        if fmt in LIMITS:
            _, digits = mm_digits(v)
            if len(digits) > LIMITS[fmt]:
                return TEXTS["range"].format(name=name, what=WHAT[key], value=shown(v, 3), format=NAMES[fmt])
    return None


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;")


def gsi_word(wi, info, sign, data, width):
    return f"{wi}{info}{sign}{data:0>{width}}"


def write(points, options):
    fmt, job, stamp = options["format"], options["job"], options["stamp"]
    if (fmt == "nikon" and any(not (0x20 <= ord(c) <= 0x7E) for c in job)) or (fmt == "jobxml" and control(job)):
        return {"error": TEXTS["job"].format(job=job, format=NAMES[fmt])}
    lines, skipped, written = [], [], 0
    if fmt == "jobxml":
        lines += ['<?xml version="1.0" encoding="UTF-8"?>', f'<JOBFile jobName="{esc(job)}" version="5.3" product="KentOS CAD" TimeStamp="{esc(stamp)}">', "  <FieldBook>"]
    if fmt == "nikon":
        lines += ["CO,Nikon RAW data format V2.00", f"CO,{job}", "CO,Dist Units: Metres"]
    for i, p in enumerate(points, 1):
        said = problem(i, p, fmt)
        if said:
            skipped.append({"index": i, "name": p["name"], "problem": said})
            continue
        written += 1
        name, code, z = p["name"], p.get("code") or "", p.get("elevation")
        e, n = shown(p["east"], 3), shown(p["north"], 3)
        zt = shown(z, 3) if z is not None else ""
        if fmt in ("gsi16", "gsi8"):
            width = LIMITS[fmt]
            words = [gsi_word("11", f"{written % 10000:04d}", "+", name, width)]
            for wi, v in (("81", p["east"]), ("82", p["north"]), ("83", z)):
                if v is not None:
                    sign, digits = mm_digits(v)
                    words.append(gsi_word(wi, "..10", sign, digits, width))
            if code:
                words.append(gsi_word("71", "....", "+", code, width))
            lines.append(("*" if fmt == "gsi16" else "") + "".join(w + " " for w in words))
        elif fmt == "gts7":
            lines.append(f"{name},{e},{n},{zt},{code}")
        elif fmt == "nikon":
            lines.append(f"UP,{name},,{n},{e},{zt},{code}")
        elif fmt == "csv":
            lines.append(f"{name},{e},{n},{zt},{code}")
        elif fmt == "jobxml":
            lines.append(f'    <PointRecord ID="{written:08X}" TimeStamp="{esc(stamp)}">')
            lines.append(f"      <Name>{esc(name)}</Name>")
            if code:
                lines.append(f"      <Code>{esc(code)}</Code>")
            lines += ["      <Method>KeyedIn</Method>", "      <Classification>Normal</Classification>", "      <Deleted>false</Deleted>", "      <Grid>"]
            lines += [f"        <North>{n}</North>", f"        <East>{e}</East>"]
            if z is not None:
                lines.append(f"        <Elevation>{zt}</Elevation>")
            lines += ["      </Grid>", "    </PointRecord>"]
    if fmt == "jobxml":
        lines += ["  </FieldBook>", "</JOBFile>"]
    return {"text": "".join(l + "\r\n" for l in lines), "written": written, "skipped": skipped}


# The stake-out points of the sample drawing's parcel (TUREF / TM36), a point without elevation or code, and the ones a
# format cannot carry.
GOOD = [
    {"name": "101/7-1", "east": 486512.352, "north": 4420187.531, "elevation": 1098.158, "code": "SINIR"},
    {"name": "101/7-2", "east": 486535.749, "north": 4420188.715, "elevation": 1098.776, "code": "SINIR"},
    {"name": "P12", "east": 486538.2334, "north": 4420218.9765, "elevation": None, "code": None},
    {"name": "K-5", "east": -12.0005, "north": 0.0004, "elevation": -1.25, "code": "AGAC"},
]
TROUBLE = [
    {"name": "", "east": 1.0, "north": 2.0, "elevation": None, "code": None},
    {"name": "ŞEHİR-1", "east": 1.0, "north": 2.0, "elevation": None, "code": None},
    {"name": "A 1", "east": 1.0, "north": 2.0, "elevation": None, "code": None},
    {"name": "A,1", "east": 1.0, "north": 2.0, "elevation": None, "code": None},
    {"name": " B2", "east": 1.0, "north": 2.0, "elevation": None, "code": None},
    {"name": "C\t3", "east": 1.0, "north": 2.0, "elevation": None, "code": None},
    {"name": "007", "east": 1.0, "north": 2.0, "elevation": None, "code": None},
    {"name": "0", "east": 1.0, "north": 2.0, "elevation": None, "code": "0"},
    {"name": "UZUN-AD-123456789", "east": 1.0, "north": 2.0, "elevation": None, "code": None},
    {"name": "AD-12345", "east": 1.0, "north": 2.0, "elevation": None, "code": "KOD-UZUN-9"},
    {"name": "D4", "east": 1.0, "north": 2.0, "elevation": None, "code": "SINIR,2"},
    {"name": "E5", "east": 1.0, "north": 2.0, "elevation": None, "code": "Çİ"},
    {"name": "F6", "east": 1.0, "north": 2.0, "elevation": None, "code": "07"},
    {"name": "G7", "east": 100000.0, "north": 2.0, "elevation": None, "code": None},
    {"name": "H8", "east": 99999.9994, "north": 2.0, "elevation": None, "code": None},
    {"name": "I<9>&\"", "east": 1.0, "north": 2.0, "elevation": None, "code": "a&b"},
]
OPTIONS = {"job": "Ada 1244", "stamp": "2026-10-04T10:15:00"}


def cases():
    out = []
    for fmt in NAMES:
        out.append((f"{NAMES[fmt]}: örnek çizimin aplikasyon noktaları", GOOD, {"format": fmt, **OPTIONS}))
        out.append((f"{NAMES[fmt]}: taşınamayan adlar, kodlar ve değerler", TROUBLE, {"format": fmt, **OPTIONS}))
    out.append(("Nikon RAW: ASCII olmayan iş adı", GOOD, {"format": "nikon", "job": "Şantiye", "stamp": OPTIONS["stamp"]}))
    out.append(("Trimble JobXML: iş adında denetim karakteri", GOOD, {"format": "jobxml", "job": "Ada\n1244", "stamp": OPTIONS["stamp"]}))
    return out


def build():
    out = [{"name": name, "points": points, "options": options, "expect": write(points, options)} for name, points, options in cases()]
    return {"format": "kentos.field-write", "version": 1, "source": "scripts/fixtures/field_write_cases.py (docs/adr/0169 §4; the formats' descriptions)", "texts": TEXTS, "cases": out}


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the file instead of writing it")
    args = parser.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
