#!/usr/bin/env python3
"""Independent reference of the Sokkia SDR field book's reading (docs/adr/0169 §1; step 5a).

Writes fixtures/field/v1/sdr.json from Sokkia's description alone ("Interfacing with the SOKKIA SDR Electronic Field
Book", software version 04-04.xx, Sokkia Technology, October 1999: chapter 3, the record formats of 3.6.1 and 3.6.2),
no KentOS code: SDR2x (4-digit point numbers, 10-character reals) and SDR33 (14-character point names, 16-character
fields) records read into the book's stations and observations.

The rules:

1. Lines end with LF, CR LF or CR; blank lines are passed over. A line that begins with STX (hex 02) or ETX (hex 03)
   frames a transmission and is passed over.
2. A record's bytes 1–2 are its type, 3–4 its derivation code; its fields are at fixed positions (counted from 1, both
   ends included), and a field past the line's end is blank (trailing blanks may be cut).
3. The header (00) begins a job: its version (5–20) beginning with "SDR33" is the SDR33 format, any other (SDR20
   V03-05) the SDR2x format. Its options: the angle unit at 41 (1 degrees, 2 gons; 3 mils and others not read), the
   distance unit at 42 (1 metres; 2 feet and others not read), the angles' direction at 46 (1; others not read). A job
   not read is said once at its header; its records are passed over until the next header. A job whose angle unit is
   not the book's (the first job's) is not read either. Records before the first header are said once and passed over.
4. Fields. SDR2x: station (02) point 5–8, north 9–18, east 19–28, elevation 29–38, instrument height 39–48; target
   (03) height 5–14; observation (09) from 5–8, to 9–12, slope distance 13–22, vertical 23–32, horizontal 33–42,
   description 43–58. SDR33: station point 5–20, north 21–36, east 37–52, elevation 53–68, instrument height 69–84;
   target height 5–20; observation from 5–20, to 21–36, slope distance 37–52, vertical 53–68, horizontal 69–84,
   description 85–100. The instrument (01) has its vertical angle option at 51 in both: 1 zenith, 2 from the horizon.
5. Texts (points, descriptions) are trimmed. A real is trimmed; blank is null; otherwise an optional minus, at least
   one digit, an optional point and digits: its exact decimal rounded once to the nearest float64. A real that is not
   so leaves its record out, said. An angle is from 0 up to a full turn (360°, 400 gon); one outside leaves its record
   out, said; a slope distance below zero too.
6. A station record (02) begins a station: its point, instrument height, north, east and elevation.
7. A target record (03) sets the target height of the observations that follow, across stations, until another or a
   new job.
8. An observation (09) with the derivation code F1, F2 or MD (raw: face one, face two, multiple distances) is read:
   its target is the "to" point (blank: not read, said), its horizontal reading the horizontal observation (null: not
   read, said), its zenith the vertical observation (null: a direction only), its slope distance, the target height
   in force, its code the description. With the vertical option 2 the zenith is a quarter turn less the reading,
   within a turn (exactly, then rounded once). Without an instrument record before it in its job the reading is taken
   for a zenith, said once at the job's first observation. An observation from a point that is not the current
   station's (or the first of a job without a station record) begins a station of that name.
9. The corrected observations (09 MC: averaged, an azimuth for a horizontal reading) and the coordinate records (08)
   are no raw observation: each kind is said once, at its first line, with its count. Other records (job, notes,
   back bearings, reductions, roads, GPS) are passed over.
"""

import argparse
import json
import re
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "field" / "v1" / "sdr.json"
TEXTS = {
    "before": "Satır {line}: SDR başlık kaydından (00) önce; başlığa kadar kayıtlar okunmadı.",
    "angleUnit": "Satır {line}: SDR işinin açı birimi ({unit}) okunmuyor; yalnız derece ve gon; iş okunmadı.",
    "distanceUnit": "Satır {line}: SDR işinin uzunluk birimi ({unit}) okunmuyor; yalnız metre; iş okunmadı.",
    "direction": "Satır {line}: SDR işinin açı yönü seçeneği ({option}) okunmuyor; yalnız 1 (sağa); iş okunmadı.",
    "mixed": "Satır {line}: SDR işinin açı birimi karnenin birimiyle aynı değil; iş okunmadı.",
    "value": "Satır {line}: “{data}” sayı değil; satır okunmadı.",
    "turn": "Satır {line}: açı “{data}” sıfırla bir tam dönüş arasında değil; satır okunmadı.",
    "negative": "Satır {line}: eğik uzunluk “{data}” sıfırdan küçük; satır okunmadı.",
    "target": "Satır {line}: gözlemin hedef noktası yok; satır okunmadı.",
    "horizontal": "Satır {line}: {target} gözleminin yatay açısı yok; satır okunmadı.",
    "zenith": "Satır {line}: işte alet kaydı (01) yok; düşey açılar başucu açısı sayıldı.",
    "corrected": "Satır {line}: düzeltilmiş gözlemler (09MC, {count} kayıt) okunmadı; ham gözlemler (F1, F2) okunur.",
    "coordinates": "Satır {line}: koordinat kayıtları (08, {count} kayıt) karne gözlemi değil; okunmadı.",
}
REAL = re.compile(r"-?[0-9]+(\.[0-9]*)?")
FULL = {"deg": 360, "grad": 400}
QUARTER = {"deg": 90, "grad": 100}
FIELDS = {
    # (format) -> record -> name -> (from, to)
    "sdr2x": {
        "02": {"point": (5, 8), "north": (9, 18), "east": (19, 28), "height": (29, 38), "instrument": (39, 48)},
        "03": {"target": (5, 14)},
        "09": {"from": (5, 8), "to": (9, 12), "slope": (13, 22), "vertical": (23, 32), "horizontal": (33, 42), "code": (43, 58)},
    },
    "sdr33": {
        "02": {"point": (5, 20), "north": (21, 36), "east": (37, 52), "height": (53, 68), "instrument": (69, 84)},
        "03": {"target": (5, 20)},
        "09": {"from": (5, 20), "to": (21, 36), "slope": (37, 52), "vertical": (53, 68), "horizontal": (69, 84), "code": (85, 100)},
    },
}


def field(line, at):
    a, b = at
    return line[a - 1 : b].strip()


class Unread(Exception):
    def __init__(self, key, data):
        super().__init__(key)
        self.key, self.data = key, data


def real(data):
    if data == "":
        return None
    if not REAL.fullmatch(data):
        raise Unread("value", data)
    return Fraction(data)


def angle(data, unit):
    v = real(data)
    if v is not None and not (0 <= v < FULL[unit]):
        raise Unread("turn", data)
    return v


def read(text):
    stations, problems = [], []
    book_unit = None
    job = None  # the job being read: its format, unit, vertical option, target height; None before a header
    skipping = False
    before_said = False
    station = None
    counts = {}  # kind -> [first line, count]
    lines = text.replace("\r\n", "\n").replace("\r", "\n").split("\n")
    for i, raw in enumerate(lines):
        line = i + 1
        if not raw.strip():
            continue
        if raw[0] in "\x02\x03":
            continue
        kind = raw[:2]
        if kind == "00":
            version = field(raw, (5, 20))
            angle_unit, distance_unit, direction = raw[40:41], raw[41:42], raw[45:46]
            skipping = True
            job = None
            station = None
            if angle_unit not in ("1", "2"):
                problems.append({"line": line, "problem": TEXTS["angleUnit"].format(line=line, unit=angle_unit or " ")})
                continue
            if distance_unit != "1":
                problems.append({"line": line, "problem": TEXTS["distanceUnit"].format(line=line, unit=distance_unit or " ")})
                continue
            if direction != "1":
                problems.append({"line": line, "problem": TEXTS["direction"].format(line=line, option=direction or " ")})
                continue
            unit = "deg" if angle_unit == "1" else "grad"
            if book_unit is None:
                book_unit = unit
            elif unit != book_unit:
                problems.append({"line": line, "problem": TEXTS["mixed"].format(line=line)})
                continue
            skipping = False
            job = {"format": "sdr33" if version.startswith("SDR33") else "sdr2x", "unit": unit, "vertical": None, "target": None, "said": False}
            continue
        if job is None:
            if not skipping and not before_said:
                problems.append({"line": line, "problem": TEXTS["before"].format(line=line)})
                before_said = True
            continue
        layout = FIELDS[job["format"]]
        try:
            if kind == "01":
                job["vertical"] = raw[50:51]
            elif kind == "02":
                f = layout["02"]
                north, east, height, instrument = (real(field(raw, f[k])) for k in ("north", "east", "height", "instrument"))
                station = {"station": field(raw, f["point"])}
                if instrument is not None:
                    station["instrumentHeight"] = float(instrument)
                for key, v in (("east", east), ("north", north), ("height", height)):
                    if v is not None:
                        station[key] = float(v)
                station["observations"] = []
                stations.append(station)
            elif kind == "03":
                job["target"] = real(field(raw, layout["03"]["target"]))
            elif kind == "09" and raw[2:4] in ("F1", "F2", "MD"):
                f = layout["09"]
                unit = job["unit"]
                hz = angle(field(raw, f["horizontal"]), unit)
                vertical = angle(field(raw, f["vertical"]), unit)
                slope = real(field(raw, f["slope"]))
                if slope is not None and slope < 0:
                    raise Unread("negative", field(raw, f["slope"]))
                target = field(raw, f["to"])
                if not target:
                    problems.append({"line": line, "problem": TEXTS["target"].format(line=line)})
                    continue
                if hz is None:
                    problems.append({"line": line, "problem": TEXTS["horizontal"].format(line=line, target=target)})
                    continue
                if job["vertical"] is None and not job["said"]:
                    problems.append({"line": line, "problem": TEXTS["zenith"].format(line=line)})
                    job["said"] = True
                zenith = vertical
                if vertical is not None and job["vertical"] == "2":
                    zenith = (QUARTER[unit] - vertical) % FULL[unit]
                source = field(raw, f["from"])
                if station is None or station["station"] != source:
                    station = {"station": source, "observations": []}
                    stations.append(station)
                o = {"target": target, "hz": float(hz)}
                if zenith is not None:
                    o["zenith"] = float(zenith)
                if slope is not None:
                    o["slope"] = float(slope)
                if job["target"] is not None:
                    o["targetHeight"] = float(job["target"])
                code = field(raw, f["code"])
                if code:
                    o["code"] = code
                o["line"] = line
                station["observations"].append(o)
            elif kind == "09" and raw[2:4] == "MC":
                counts.setdefault("corrected", [line, 0])[1] += 1
            elif kind == "08":
                counts.setdefault("coordinates", [line, 0])[1] += 1
        except Unread as e:
            problems.append({"line": line, "problem": TEXTS[e.key].format(line=line, data=e.data)})
    for key in ("corrected", "coordinates"):
        if key in counts:
            first, count = counts[key]
            problems.append({"line": first, "problem": TEXTS[key].format(line=first, count=count)})
    problems.sort(key=lambda p: p["line"])
    return {"unit": book_unit, "stations": stations, "problems": problems}


def at(width, value, right=False):
    s = str(value)
    assert len(s) <= width, (value, width)
    return s.rjust(width) if right else s.ljust(width)


def header(version, units):
    # 00 NM, version (16), serial (4), date and time (16), six options.
    return "00NM" + at(16, version) + "0000" + at(16, "04-Oct-26 10:00") + units


def sdr33(kind, deriv, *fields):
    return kind + deriv + "".join(at(16, f) for f in fields)


def cases():
    # SDR33: 14-character names, 16-character fields, gons; an instrument with zenith angles, a station, a back sight
    # direction (no distance, no vertical), a pair of faces, a code, a target height that changes across stations.
    s33 = "\r\n".join(
        [
            "\x02",
            header("SDR33 V04-04.02", "211111"),
            "10NM" + at(16, "KARNE") + "121111",
            "01NM:" + at(16, "SET") + "000000" + at(16, "SET") + "000000" + "31" + at(16, "0.00000000") + at(16, "0.00000000") + at(16, "0.00000000"),
            sdr33("02", "TP", "ST1", "4521800.0000", "412350.0000", "105.2000", "1.5520", "IST"),
            sdr33("07", "TP", "ST1", "P2", "12.34560000", "0.00120000"),
            sdr33("03", "NM", "1.70000000"),
            sdr33("09", "F1", "ST1", "P2", "", "", "0.00120000", "BS"),
            sdr33("09", "F1", "ST1", "101", "63.2140", "101.23450000", "87.43210000", "BINA"),
            sdr33("09", "F2", "ST1", "101", "63.2160", "298.76710000", "287.43370000", "BINA"),
            sdr33("03", "NM", "2.00000000"),
            sdr33("09", "MD", "ST1", "102", "82.4410", "100.44000000", "120.55500000"),
            sdr33("09", "MC", "ST1", "101", "63.2150", "101.23370000", "112.33333333"),
            sdr33("08", "TP", "101", "4521850.1", "412410.2", "104.1", "BINA"),
            sdr33("02", "TP", "ST2", "4521742.2080", "412410.5120", "104.8750", "1.4800"),
            sdr33("09", "F1", "ST2", "ST1", "83.3920", "100.33400000", "0.00000000"),
            "\x0309436",
        ]
    ) + "\r\n"
    # SDR2x: 4-digit points, 10-character reals written edge to edge as instruments write them, degrees; an instrument
    # reading from the horizon; coordinates (08) and a corrected observation (MC) said once each.
    s2x = "\n".join(
        [
            header("SDR20 V03-05", "113111"),
            "10NM20261004",
            "01NM:" + at(16, "") + "000000" + at(16, "") + "00000032" + "0.00000000" * 3,
            "02TP0001" + "0.00000000" + "0.00000000" + "100.000000" + "1.59500000",
            "03NM1.47400000",
            "09F10001999928.95000005.27500000121.855555",
            "09F1000110011234.56781355.9888881.00000000BAUM",
            "09F2000110011234.56791184.011111181.000000",
            "08TP100029.1755686-4.00952570.00963371FIT-TEST",
            "08TP10024.594338146.41467510-0.4376595FIT-TEST2",
            "09MC000110011234.5678694.01111101.00000000",
        ]
    ) + "\n"
    # A book without an instrument record (its vertical readings taken for zeniths, said once), records before the
    # header, a station by its observations' "from" point.
    bare = "\n".join(
        [
            "09F1000110011234.56789.0000000.000000",
            "13NMBEFORE THE HEADER",
            header("SDR20 V03-05", "211111"),
            "09F10007000810.0000000100.000000200.000000",
            "09F10007000910.0000000101.000000201.000000",
        ]
    ) + "\n"
    # Jobs not read: mils, feet, the angles' direction, another unit than the book's; then one read; values not read.
    broken = "\n".join(
        [
            header("SDR33 V04-04.02", "311111"),
            sdr33("09", "F1", "S1", "P1", "10.0", "90.0", "10.0"),
            header("SDR33 V04-04.02", "121111"),
            header("SDR33 V04-04.02", "111112"),
            header("SDR33 V04-04.02", "111111"),
            "01NM:" + at(16, "SET") + "000000" + at(16, "SET") + "000000" + "31",
            sdr33("09", "F1", "S1", "P1", "10.0", "90.0", "10.0"),
            header("SDR33 V04-04.02", "211111"),
            sdr33("09", "F1", "S1", "P2", "10.0", "90.0", "10.0"),
            header("SDR33 V04-04.02", "111111"),
            "01NM:" + at(16, "SET") + "000000" + at(16, "SET") + "000000" + "31",
            sdr33("09", "F1", "S1", "P3", "12a", "90.0", "10.0"),
            sdr33("09", "F1", "S1", "P4", "10.0", "360.0", "10.0"),
            sdr33("09", "F1", "S1", "P5", "-1.0", "90.0", "10.0"),
            sdr33("09", "F1", "S1", "", "10.0", "90.0", "10.0"),
            sdr33("09", "F1", "S1", "P7", "10.0", "90.0", ""),
            sdr33("09", "F1", "S1", "P8", "10.0", "90.0", ".5"),
            sdr33("09", "F1", "S1", "P9", "10", "90.", "+1.0"),
            sdr33("09", "F1", "S1", "P10", "10", "90", "359.99999999"),
            sdr33("02", "TP", "S2", "1e3", "2.0", "3.0", "1.5"),
            "09F1" + at(16, "S1") + at(16, "P11") + at(16, "5.0"),
        ]
    ) + "\n"
    return [
        ("SDR33: 14 karakterlik adlar, gon, iki durum, kod, prizma yüksekliği", s33),
        ("SDR2x: 4 haneli numaralar, bitişik alanlar, derece, ufuktan düşey açı, koordinat ve MC kayıtları", s2x),
        ("alet kaydı yok, başlıktan önce kayıtlar, istasyonsuz gözlemler", bare),
        ("okunmayan işler (mil, ayak, açı yönü, başka birim) ve okunmayan değerler", broken),
    ]


def build():
    out = [{"name": name, "text": text, "expect": read(text)} for name, text in cases()]
    return {"format": "kentos.field-sdr", "version": 1, "source": "scripts/fixtures/field_sdr_cases.py (docs/adr/0169 §1; Interfacing with the SOKKIA SDR Electronic Field Book, 1999)", "texts": TEXTS, "cases": out}


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
