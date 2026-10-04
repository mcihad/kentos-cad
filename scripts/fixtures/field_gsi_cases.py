#!/usr/bin/env python3
"""Independent reference of the Leica GSI field book's reading (docs/adr/0169 §1; step 2).

Writes fixtures/field/v1/gsi.json from Leica's GSI description alone ("GSI ONLINE for Leica TPS and DNA", Leica
Geosystems, 2003: the data word, its units digit, the word indexes), no KentOS code: GSI-8 and GSI-16 blocks read into
the book's stations and observations.

The rules:

1. Every line that is not blank is a block; one that begins with "*" is GSI-16. A block's words are separated by blanks.
2. A word: its first two characters are the word index (WI), the next four its information (the fourth of them, the
   word's sixth character, the units), the seventh its sign (+ or −), the rest its data. A word shorter than eight
   characters, or without a sign there, is no word: the block is not read, its line says so.
3. Texts (WI 11 point, WI 16 station point, WI 71 remark as the code): the data without its leading zeros (a zero
   alone stays).
4. Angles (WI 21 horizontal reading, WI 22 zenith): units 2 (400 gon) and 3 (360°, decimal) carry five decimals;
   4 (360°, sexagesimal) is DDD..MMSSs, the last digit tenths of a second, minutes and seconds below 60. The book's
   unit is the first angle's (gon, or degrees for both 3 and 4); another unit's angle is turned into it. Units 5 (mil)
   and others: the block is not read; nor is an angle whose data is not ASCII digits read so, nor one of more than a
   full turn (400 gon, 360°). An angle is its exact value rounded once to the nearest float64.
5. Lengths (WI 31 slope distance, WI 32 horizontal distance, 84–86 station coordinates, 87 reflector height, 88
   instrument height): units 0 the last digit a millimetre, 6 a tenth and 8 a hundredth of one; the data ASCII digits.
   Feet (1, 7) and others: the block is not read.
6. A block with WI 21 is an observation: its target is WI 11 (none: not read), its zenith WI 22, its slope distance WI
   31, its reflector height WI 87 (none: the station's last one), its code WI 71. A slope distance missing where WI 32
   is: said, and the observation is a direction only.
7. A block without WI 21 and with 84, 85, 86 or 88 starts a station: its name WI 16 or else WI 11, its instrument
   height WI 88, its coordinates 84 (east), 85 (north), 86 (height). Other blocks (code blocks, settings) are passed
   over.
8. An observation before any station is the book's first station's, unnamed.
"""

import argparse
import json
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "field" / "v1" / "gsi.json"
TEXTS = {
    "word": "Satır {line}: “{word}” GSI sözcüğü değil; satır okunmadı.",
    "unit": "Satır {line}: WI {wi} sözcüğünün birimi ({unit}) okunmuyor; yalnız metre, gon ve derece; satır okunmadı.",
    "value": "Satır {line}: WI {wi} sözcüğünün değeri “{data}” okunamadı; satır okunmadı.",
    "turn": "Satır {line}: WI {wi} sözcüğünün açısı “{data}” bir tam dönüşten büyük; satır okunmadı.",
    "target": "Satır {line}: ölçünün nokta numarası (WI 11) yok; satır okunmadı.",
    "horizontal": "Satır {line}: {target} noktasının eğik uzunluğu yok (yalnız WI 32 yatay uzunluk); nokta doğrultu olarak okundu.",
}
LENGTH_SCALE = {"0": 1000, "6": 10000, "8": 100000}


def text_value(data):
    s = data.lstrip("0")
    return s if s else "0"


def digits(data):
    return data.isascii() and data.isdigit()


def angle(data, unit, sign):
    """The angle in its own unit, gon or degrees: (value, 'grad' | 'deg'), "unit", "value" or "turn"."""
    if unit not in ("2", "3", "4"):
        return "unit"
    if not digits(data):
        return "value"
    if unit in ("2", "3"):
        v = Fraction(int(data), 100000)
        kind = "grad" if unit == "2" else "deg"
    else:
        if len(data) < 6:
            return "value"
        d, mm, ss, t = int(data[:-5]), int(data[-5:-3]), int(data[-3:-1]), int(data[-1])
        if mm >= 60 or ss >= 60:
            return "value"
        v = d + Fraction(mm, 60) + (ss + Fraction(t, 10)) / 3600
        kind = "deg"
    if v > (400 if kind == "grad" else 360):
        return "turn"
    return (-v if sign == "-" else v), kind


def length(data, unit, sign):
    if unit not in LENGTH_SCALE:
        return "unit"
    if not digits(data):
        return "value"
    v = Fraction(int(data), LENGTH_SCALE[unit])
    return -v if sign == "-" else v


def read(text):
    stations, problems = [], []
    book_unit = None
    current = None
    last_th = None
    for i, raw in enumerate(text.replace("\r\n", "\n").replace("\r", "\n").split("\n")):
        line = i + 1
        block = raw.strip()
        if not block:
            continue
        if block.startswith("*"):
            block = block[1:]
        words = {}
        bad = None
        for w in block.split():
            if len(w) < 8 or w[6] not in "+-":
                bad = TEXTS["word"].format(line=line, word=w)
                break
            words.setdefault(w[:2], w)
        if bad:
            problems.append({"line": line, "problem": bad})
            continue

        def value(wi, kind):
            w = words.get(wi)
            if w is None:
                return None, None
            unit, sign, data = w[5], w[6], w[7:]
            if kind == "text":
                return text_value(data), None
            got = angle(data, unit, sign) if kind == "angle" else length(data, unit, sign)
            if got == "unit":
                return None, TEXTS["unit"].format(line=line, wi=wi, unit=unit)
            if got in ("value", "turn"):
                return None, TEXTS[got].format(line=line, wi=wi, data=data)
            return got, None

        read_values, err = {}, None
        for wi, kind in [("11", "text"), ("16", "text"), ("71", "text"), ("21", "angle"), ("22", "angle"), ("31", "length"), ("32", "length"), ("84", "length"), ("85", "length"), ("86", "length"), ("87", "length"), ("88", "length")]:
            v, e = value(wi, kind)
            if e:
                err = e
                break
            if v is not None:
                read_values[wi] = v
        if err:
            problems.append({"line": line, "problem": err})
            continue

        def in_book(a):
            nonlocal book_unit
            v, kind = a
            if book_unit is None:
                book_unit = kind
            if kind == book_unit:
                return v
            return v * Fraction(9, 10) if kind == "grad" else v * Fraction(10, 9)

        if "21" in read_values:
            target = read_values.get("11")
            if target is None:
                problems.append({"line": line, "problem": TEXTS["target"].format(line=line)})
                continue
            if current is None:
                current = {"station": "", "observations": []}
                stations.append(current)
                last_th = None
            o = {"target": target, "hz": float(in_book(read_values["21"]))}
            if "22" in read_values:
                o["zenith"] = float(in_book(read_values["22"]))
            if "31" in read_values:
                o["slope"] = float(read_values["31"])
            elif "32" in read_values:
                problems.append({"line": line, "problem": TEXTS["horizontal"].format(line=line, target=target)})
            th = read_values.get("87", last_th)
            last_th = th
            if th is not None:
                o["targetHeight"] = float(th)
            if "71" in read_values:
                o["code"] = read_values["71"]
            o["line"] = line
            current["observations"].append(o)
        elif any(k in read_values for k in ("84", "85", "86", "88")):
            current = {"station": read_values.get("16", read_values.get("11", ""))}
            if "88" in read_values:
                current["instrumentHeight"] = float(read_values["88"])
            for wi, key in (("84", "east"), ("85", "north"), ("86", "height")):
                if wi in read_values:
                    current[key] = float(read_values[wi])
            current["observations"] = []
            stations.append(current)
            last_th = None
    return {"unit": book_unit, "stations": stations, "problems": problems}


def gsi8(wi, info, sign, data):
    return f"{wi}{info}{sign}{data:>08}"


def gsi16(wi, info, sign, data):
    return f"{wi}{info}{sign}{data:>016}"


def cases():
    g8 = "\n".join(
        [
            " ".join([gsi8("11", "0001", "+", "00000ST1"), gsi8("84", "..10", "+", "12345678"), gsi8("85", "..10", "+", "45678901"), gsi8("86", "..10", "+", "00105200"), gsi8("88", "..10", "+", "00001552")]),
            " ".join([gsi8("11", "0002", "+", "000000P2"), gsi8("21", ".322", "+", "00000120"), gsi8("22", ".322", "+", "09987650"), gsi8("31", "..00", "+", "00245678"), gsi8("87", "..10", "+", "00001700")]),
            " ".join([gsi8("11", "0003", "+", "00000101"), gsi8("21", ".322", "+", "08743210"), gsi8("22", ".322", "+", "10123450"), gsi8("31", "..00", "+", "00063214"), gsi8("71", "....", "+", "0000BINA")]),
            " ".join([gsi8("11", "0004", "+", "00000101"), gsi8("21", ".322", "+", "28743370"), gsi8("22", ".322", "+", "29876710"), gsi8("31", "..00", "+", "00063216")]),
        ]
    ) + "\n"
    g16 = "\r\n".join(
        [
            "*" + " ".join([gsi16("11", "0001", "+", "S1"), gsi16("88", "..18", "+", "160000")]),
            "*" + " ".join([gsi16("11", "0002", "+", "A"), gsi16("21", ".024", "+", "4512340"), gsi16("22", ".024", "+", "8930000"), gsi16("31", "..08", "+", "51234567"), gsi16("87", "..18", "+", "200000")]),
            "*" + " ".join([gsi16("11", "0003", "+", "B"), gsi16("21", ".023", "+", "4512345"), gsi16("22", ".023", "+", "8950000"), gsi16("31", "..06", "+", "5123456")]),
            "*" + " ".join([gsi16("11", "0004", "+", "C"), gsi16("21", ".022", "+", "5000000"), gsi16("22", ".022", "+", "10000000")]),
        ]
    ) + "\r\n"
    feet = " ".join([gsi8("11", "0001", "+", "00000ST1"), gsi8("88", "..11", "+", "00005000")]) + "\n"
    broken = "\n".join(
        [
            " ".join([gsi8("11", "0001", "+", "000000P1"), gsi8("21", ".322", "+", "00000000"), "22.324"]),
            " ".join([gsi8("21", ".322", "+", "00000000"), gsi8("22", ".322", "+", "10000000")]),
            " ".join([gsi8("11", "0003", "+", "000000P3"), gsi8("21", ".325", "+", "00001000")]),
            " ".join([gsi8("11", "0004", "+", "000000P4"), gsi8("21", ".322", "+", "00002000"), gsi8("22", ".322", "+", "10000000"), gsi8("32", "..00", "+", "00012345")]),
            " ".join([gsi8("41", "0005", "+", "00000013"), gsi8("42", "....", "+", "000TREES")]),
            " ".join([gsi8("11", "0006", "+", "000000P6"), gsi8("21", ".324", "+", "01275000")]),
            " ".join([gsi8("11", "0007", "+", "000000P7"), gsi8("21", ".322", "+", "50000000")]),
            " ".join([gsi8("11", "0008", "+", "000000P8"), gsi8("22", ".324", "+", "36100000")]),
            " ".join([gsi8("16", "0009", "+", "000000S9"), gsi8("86", "..10", "-", "00001000"), gsi8("88", "..10", "+", "00001500")]),
            " ".join([gsi8("11", "0010", "+", "00000P10"), gsi8("21", ".322", "-", "00500000"), gsi8("22", ".322", "+", "10000000"), gsi8("31", "..00", "+", "00010000")]),
            " ".join([gsi8("11", "0011", "+", "00000P11"), gsi8("21", ".322", "+", "0012345A")]),
        ]
    ) + "\n"
    return [
        ("GSI-8, istasyon ve iki durum (gon, mm)", g8),
        ("GSI-16, karışık birimler (derece, DMS ve gon; 1/100 mm ve 1/10 mm)", g16),
        ("ayak birimi", feet),
        ("bozuk sözcük, numarasız ölçü, mil, yalnız yatay uzunluk, kod bloğu, tam dönüşten büyük açı, eksi değerler", broken),
    ]


def build():
    out = [{"name": name, "text": text, "expect": read(text)} for name, text in cases()]
    return {"format": "kentos.field-gsi", "version": 1, "source": "scripts/fixtures/field_gsi_cases.py (docs/adr/0169 §1; Leica GSI ONLINE, 2003)", "texts": TEXTS, "cases": out}


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
