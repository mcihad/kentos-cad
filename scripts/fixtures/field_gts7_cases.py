#!/usr/bin/env python3
"""Independent reference of the Topcon GTS-7 field book's reading (docs/adr/0169 §1; step 5b).

Writes fixtures/field/v1/gts7.json from Topcon's description alone (Topcon Link Reference Manual, P/N 7010-0522,
Appendix C "GTS-7 Raw Format", with its sample file), no KentOS code: GTS-7 records read into the book's stations and
observations.

The rules:

1. Lines end with LF, CR LF or CR; blank lines are passed over. A record is a control word, blanks, then its fields
   separated by commas, each trimmed. A line whose control word is none of GTS-7's (the version line, TTools v1.0) is
   passed over, as are JOB, DATE, NAME, INST, SCALE, ATMOS, TEMP, BKB, CTL and NOTE.
2. UNITS: its first field's first letter M (metres; F feet, or any other, is not read), its second's D (degrees,
   written DDD.MMSS: the fraction's first two digits minutes, the next two seconds, the rest the seconds' fraction;
   fewer digits are zeros) or G (gon, decimal). A UNITS not read is said at its line and the observations after it
   are passed over until a UNITS that is read, as after one whose angle unit is not the book's (the first UNITS');
   observations before any UNITS are said once and passed over.
3. A number: an optional sign, digits with an optional point, or a point and digits, at most 30 digits; its exact value
   rounded once to the nearest float64. A number that is not so, a DDD.MMSS angle whose minutes or seconds are 60 or more, or an
   angle of more than a full turn (its size) leaves its record out, said. A horizontal reading below zero is turned
   into its turn (−37.2644 is 322°33'16"); a zenith below zero is not read.
4. STN ptno, ins ht, stn id: begins a station named ptno, its instrument height. XYZ easting, northing, elevation
   right after STN gives its coordinates; an XYZ after any other record is a point's computed coordinates, passed
   over.
5. BS ptno[, target height], FS and SS ptno, target height, pt code[, string]: the point the next measurements are
   of, its target height (none given: the station's last), its code (FS, SS).
6. HV HA, VA and SD HA, VA, SD: an observation of the point named last (none in the station: not read, said): its
   horizontal reading (none: not read, said), its zenith, its slope distance (SD). HD HA, HD, VD is a reduced measurement: not read, said.
7. OFFSET radial, tangential, vertical: any of them not zero leaves the observation before it out, said.
8. An observation before any station is the book's first station's, unnamed.
"""

import argparse
import json
import re
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "field" / "v1" / "gts7.json"
TEXTS = {
    "units": "Satır {line}: GTS-7 UNITS kaydı “{data}” okunmuyor; yalnız metre (M) ile derece (D) ya da gon (G); ölçüler okunmadı.",
    "mixed": "Satır {line}: GTS-7 UNITS kaydının açı birimi karnenin birimiyle aynı değil; ölçüler okunmadı.",
    "before": "Satır {line}: GTS-7 UNITS kaydından önce; birim bilinmediğinden ölçüler okunmadı.",
    "value": "Satır {line}: “{data}” sayı değil; satır okunmadı.",
    "dms": "Satır {line}: açı “{data}” DDD.MMSS değil (dakika ya da saniye 60'tan büyük); satır okunmadı.",
    "turn": "Satır {line}: açı “{data}” bir tam dönüşten büyük; satır okunmadı.",
    "zenith": "Satır {line}: başucu açısı “{data}” sıfırdan küçük; satır okunmadı.",
    "negative": "Satır {line}: eğik uzunluk “{data}” sıfırdan küçük; satır okunmadı.",
    "point": "Satır {line}: ölçüden önce nokta kaydı (BS, FS, SS) yok; satır okunmadı.",
    "horizontal": "Satır {line}: {target} gözleminin yatay açısı yok; satır okunmadı.",
    "reduced": "Satır {line}: HD kaydı indirgenmiş ölçü (yatay uzunluk, kot farkı); ham gözlem değil, satır okunmadı.",
    "offset": "Satır {line}: dışmerkez ölçü (OFFSET) okunmuyor; {target} gözlemi (satır {at}) okunmadı.",
}
NUMBER = re.compile(r"[+-]?([0-9]+(\.[0-9]*)?|\.[0-9]+)")
WORDS = {"GTS-700", "JOB", "DATE", "NAME", "INST", "UNITS", "SCALE", "ATMOS", "TEMP", "STN", "XYZ", "BKB", "BS", "FS", "SS", "CTL", "HV", "SD", "HD", "OFFSET", "NOTE"}
FULL = {"deg": 360, "grad": 400}


class Unread(Exception):
    def __init__(self, key, data):
        super().__init__(key)
        self.key, self.data = key, data


def number(data):
    if data == "":
        return None
    if not NUMBER.fullmatch(data) or sum(c.isdigit() for c in data) > 30:
        raise Unread("value", data)
    return Fraction(data)


def dms(data):
    """A DDD.MMSS angle's exact value in degrees."""
    if not NUMBER.fullmatch(data) or sum(c.isdigit() for c in data) > 30:
        raise Unread("value", data)
    sign = -1 if data.startswith("-") else 1
    body = data.lstrip("+-")
    whole, _, frac = body.partition(".")
    frac = frac.ljust(4, "0")
    minutes, seconds, rest = int(frac[:2]), int(frac[2:4]), frac[4:]
    if minutes >= 60 or seconds >= 60:
        raise Unread("dms", data)
    secs = Fraction(int(whole or "0") * 3600 + minutes * 60 + seconds) + (Fraction(int(rest), 10 ** len(rest)) if rest else 0)
    return sign * secs / 3600


def angle(data, unit, horizontal):
    if data == "":
        return None
    v = dms(data) if unit == "deg" else number(data)
    if abs(v) > FULL[unit]:
        raise Unread("turn", data)
    if v < 0:
        if not horizontal:
            raise Unread("zenith", data)
        v += FULL[unit]
    return v


def read(text):
    stations, problems = [], []
    unit = None  # the angle unit in force: "deg", "grad", "none" (a UNITS not read), None (no UNITS yet)
    book_unit = None
    before_said = False
    station = None
    point = None  # the point the next measurements are of: name, target height, code
    last_th = None
    previous = None  # the record before: its word
    last_obs = None  # the observation just read: its line and target
    for i, raw in enumerate(text.replace("\r\n", "\n").replace("\r", "\n").split("\n")):
        line = i + 1
        s = raw.strip()
        if not s:
            continue
        parts = s.split(None, 1)
        word = parts[0].upper() if parts[0].upper() in WORDS else None
        fields = [f.strip() for f in parts[1].split(",")] if len(parts) > 1 else []
        get = lambda k: fields[k] if k < len(fields) else ""
        here = previous
        previous = word
        observation = last_obs
        last_obs = None
        if word is None:
            continue
        try:
            if word == "UNITS":
                d, a = get(0)[:1].upper(), get(1)[:1].upper()
                if d == "M" and a in ("D", "G"):
                    unit = "deg" if a == "D" else "grad"
                    if book_unit is None:
                        book_unit = unit
                    elif book_unit != unit:
                        unit = "none"
                        problems.append({"line": line, "problem": TEXTS["mixed"].format(line=line)})
                else:
                    unit = "none"
                    problems.append({"line": line, "problem": TEXTS["units"].format(line=line, data=", ".join(fields))})
            elif word == "STN":
                hi = number(get(1))
                station = {"station": get(0)}
                if hi is not None:
                    station["instrumentHeight"] = float(hi)
                station["observations"] = []
                stations.append(station)
                point, last_th = None, None
            elif word == "XYZ":
                if here == "STN" and station is not None:
                    e, n, z = number(get(0)), number(get(1)), number(get(2))
                    obs = station.pop("observations")
                    for key, v in (("east", e), ("north", n), ("height", z)):
                        if v is not None:
                            station[key] = float(v)
                    station["observations"] = obs
            elif word in ("BS", "FS", "SS"):
                th = number(get(1))
                if th is not None:
                    last_th = th
                point = {"name": get(0), "th": last_th, "code": get(2) if word != "BS" else ""}
            elif word in ("HV", "SD", "HD"):
                if unit is None:
                    if not before_said:
                        problems.append({"line": line, "problem": TEXTS["before"].format(line=line)})
                        before_said = True
                    continue
                if unit == "none":
                    continue
                if word == "HD":
                    problems.append({"line": line, "problem": TEXTS["reduced"].format(line=line)})
                    continue
                hz = angle(get(0), unit, True)
                zenith = angle(get(1), unit, False)
                slope = number(get(2)) if word == "SD" else None
                if slope is not None and slope < 0:
                    raise Unread("negative", get(2))
                if point is None:
                    problems.append({"line": line, "problem": TEXTS["point"].format(line=line)})
                    continue
                if hz is None:
                    problems.append({"line": line, "problem": TEXTS["horizontal"].format(line=line, target=point["name"])})
                    continue
                if station is None:
                    station = {"station": "", "observations": []}
                    stations.append(station)
                o = {"target": point["name"], "hz": float(hz)}
                if zenith is not None:
                    o["zenith"] = float(zenith)
                if slope is not None:
                    o["slope"] = float(slope)
                if point["th"] is not None:
                    o["targetHeight"] = float(point["th"])
                if point["code"]:
                    o["code"] = point["code"]
                o["line"] = line
                station["observations"].append(o)
                last_obs = (station, o)
            elif word == "OFFSET":
                offsets = [number(get(k)) for k in range(3)]
                if observation is not None and any(v not in (None, 0) for v in offsets):
                    st, o = observation
                    st["observations"].remove(o)
                    problems.append({"line": line, "problem": TEXTS["offset"].format(line=line, target=o["target"], at=o["line"])})
        except Unread as e:
            problems.append({"line": line, "problem": TEXTS[e.key].format(line=line, data=e.data)})
    return {"unit": book_unit, "stations": stations, "problems": problems}


def cases():
    # Topcon Link's own sample (Appendix C), its first station: degrees in DDD.MMSS, horizontal readings below zero,
    # XYZ records after BKB and SS (a point's coordinates, passed over).
    topcon = "\n".join(
        [
            "TTools v1.0",
            "JOB           C:\\Download\\777.raw,Comment",
            "NAME          TopconTools",
            "INST          TS",
            "UNITS         M,D",
            "SCALE         1.000000,1.000000,0.000000",
            "DATE          00/00/00,00:00",
            "TEMP          0.000,000",
            "STN           MARK,1.52000,STAT",
            "BKB           ST1,0.0000,322.33160",
            "XYZ           13.85600,7.04700,-0.25800",
            "BS            ST1,1.60000",
            "SD            -37.26440,97.57060,4.90200",
            "SS            ST1,1.60000,STAT",
            "SD            -37.26440,97.57060,4.90400",
            "XYZ           13.85600,7.04700,-0.25800",
            "SS            ST2,1.60000,STAT",
            "SD            7.56170,97.13460,4.95600",
            "XYZ           14.87000,10.67900,-0.20400",
            "SS            1,1.60000,TREE",
            "SD            91.02230,78.18030,3.44800",
            "SS            4,1.60000,TREE",
            "SD            -142.30250,90.11250,3.89200",
        ]
    ) + "\n"
    # Gon, a station with its coordinates, both faces, an angle-only measurement, a target height left out on BS,
    # an OFFSET of zero and one that is not.
    gon = "\r\n".join(
        [
            "GTS-700",
            "UNITS M,G",
            "STN ST1,1.552,IST",
            "XYZ 412350.000,4521800.000,105.200",
            "BS P2,1.700",
            "HV 0.0012,99.8765",
            "SS 101,1.700,BINA",
            "SD 87.4321,101.2345,63.214",
            "SD 287.4337,298.7671,63.216",
            "OFFSET 0.000,0.000,0.000",
            "SS 102,2.000,",
            "SD 120.5550,100.4400,82.441",
            "OFFSET 0.150,0.000,0.000",
            "STN ST2,1.480",
            "BS ST1",
            "SD 0.0000,100.3340,83.392",
        ]
    ) + "\r\n"
    # Observations before UNITS, units not read (feet, mils), a UNITS of another angle unit than the book's,
    # reduced HD, measurements without a point, bad numbers and angles.
    broken = "\n".join(
        [
            "STN S1,1.5",
            "SS P1,1.6,",
            "SD 10.0000,90.0000,10.0",
            "UNITS F,D",
            "SD 10.0000,90.0000,10.0",
            "UNITS M,D",
            "STN S2,1.5",
            "SD 10.0000,90.0000,10.0",
            "SS P2,1.6,",
            "HD 10.0000,9.9,0.5",
            "SD 10.6000,90.0000,10.0",
            "SD 10.0060,90.0000,10.0",
            "SD 360.0001,90.0000,10.0",
            "SD 10.0000,-1.0000,10.0",
            "SD 10.0000,90.0000,-2.0",
            "SD 10.0000,90.0000,1e3",
            "SD 10.0000,90.0000,.5",
            "SD 10.30,90.,+7",
            "SD ,90.0000,10.0",
            "UNITS M,G",
            "SD 10.0000,100.0000,10.0",
        ]
    ) + "\n"
    return [
        ("Topcon Link'in örneği: derece (DDD.MMSS), eksi yatay açılar, XYZ kayıtları", topcon),
        ("gon, istasyon koordinatları, iki durum, yalnız açı, OFFSET", gon),
        ("UNITS'ten önce ölçü, okunmayan birimler, HD, noktasız ölçü, bozuk sayılar ve açılar", broken),
    ]


def build():
    out = [{"name": name, "text": text, "expect": read(text)} for name, text in cases()]
    return {"format": "kentos.field-gts7", "version": 1, "source": "scripts/fixtures/field_gts7_cases.py (docs/adr/0169 §1; Topcon Link Reference Manual, Appendix C)", "texts": TEXTS, "cases": out}


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
