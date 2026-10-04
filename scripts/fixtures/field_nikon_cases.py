#!/usr/bin/env python3
"""Independent reference of the Nikon RAW field book's reading (docs/adr/0169 §1; step 5d).

Writes fixtures/field/v1/nikon.json from Nikon's description alone (Total Station Nivo Series Instruction Manual,
Nikon-Trimble: "Nikon raw record formats" and "Data examples", Nikon RAW data format V2.00), no KentOS code: the
records read into the book's stations and observations.

The rules:

1. Lines end with LF, CR LF or CR; blank lines are passed over. A record's fields are separated by commas, each
   trimmed; the first is the record's type.
2. Comment records (CO) of the form "Key: value" set the book's units: "Dist Units" (a value beginning with "Met",
   metres; any other is not read), "Angle Units" (DDDMMSS: degrees written DDD.MMSS, the fraction's first two digits
   minutes, the next two seconds, the rest the seconds' fraction, fewer digits zeros; Gon, Gons, Grad or Grads: gon,
   decimal; any other is not read), "Zero VA" (Zenith, or Horizon: the vertical angle from the horizon; any other is
   not read). A unit not read is said at its line and the observations after it are passed over. Observations before
   the distance and angle units are known are said once and passed over; without a Zero VA the vertical angles are
   taken for zeniths, said once at the first observation. Other comments are passed over.
3. A number: an optional sign, digits with an optional point, or a point and digits, at most 30 digits; its exact value
   rounded once to the nearest float64. A number that is not so, a DDD.MMSS angle whose minutes or seconds are 60 or more, or an angle
   of more than a full turn leaves its record out, said. A horizontal reading below zero is turned into its turn; a
   zenith below zero (after the horizon's turn) is not read, nor a slope distance below zero.
4. Coordinate records (UP, MP, CC, RE: pt, pt id, northing, easting, elevation, code) give a point's coordinates,
   the last record of a name its own; they are not observations.
5. ST stnpt, stnid, bspt, bsid, hi, bsazim, bsha begins a station: named stnpt (or stnid), its instrument height hi,
   its coordinates the last coordinate record of its name before it, its back sight bspt (or bsid).
6. Observations: F1 and F2 (pt, ht, sd, ha, va, time; pt empty: the station's back sight), SS (pt, ht, sd, ha, va,
   time, code), CP (pt, pt id, ht, sd, ha, va, time, code), SO (pt, original pt, ht, sd, ha, va, time): the target pt
   (none at all: not read, said), the target height ht (empty: the station's last), the slope distance sd (empty: a
   direction only), the horizontal reading ha (empty: not read, said), the zenith from va (empty: none; from the
   horizon, a quarter turn less va within a turn, exactly), the code.
7. An observation before any station is the book's first station's, unnamed.
"""

import argparse
import json
import re
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "field" / "v1" / "nikon.json"
TEXTS = {
    "distance": "Satır {line}: Nikon RAW uzunluk birimi “{data}” okunmuyor; yalnız metre; ölçüler okunmadı.",
    "angles": "Satır {line}: Nikon RAW açı birimi “{data}” okunmuyor; yalnız DDDMMSS ve gon; ölçüler okunmadı.",
    "vertical": "Satır {line}: Nikon RAW düşey açı başlangıcı “{data}” okunmuyor; yalnız Zenith ve Horizon; ölçüler okunmadı.",
    "mixed": "Satır {line}: Nikon RAW açı birimi karnenin birimiyle aynı değil; ölçüler okunmadı.",
    "before": "Satır {line}: Nikon RAW birimleri (CO,Dist Units ve CO,Angle Units) bu satırdan önce yazılı değil; ölçüler okunmadı.",
    "zenith": "Satır {line}: Nikon RAW düşey açı başlangıcı (CO,Zero VA) yazılı değil; düşey açılar başucu açısı sayıldı.",
    "value": "Satır {line}: “{data}” sayı değil; satır okunmadı.",
    "dms": "Satır {line}: açı “{data}” DDD.MMSS değil (dakika ya da saniye 60'tan büyük); satır okunmadı.",
    "turn": "Satır {line}: açı “{data}” bir tam dönüşten büyük; satır okunmadı.",
    "below": "Satır {line}: başucu açısı “{data}” sıfırdan küçük; satır okunmadı.",
    "negative": "Satır {line}: eğik uzunluk “{data}” sıfırdan küçük; satır okunmadı.",
    "target": "Satır {line}: gözlemin hedef noktası yok; satır okunmadı.",
    "horizontal": "Satır {line}: {target} gözleminin yatay açısı yok; satır okunmadı.",
}
NUMBER = re.compile(r"[+-]?([0-9]+(\.[0-9]*)?|\.[0-9]+)")
FULL = {"deg": 360, "grad": 400}
QUARTER = {"deg": 90, "grad": 100}
COORDS = {"UP", "MP", "CC", "RE"}
# The record's fields: target (and the alternative), target height, slope distance, horizontal, vertical, code.
OBS = {
    "F1": (1, None, 2, 3, 4, 5, None),
    "F2": (1, None, 2, 3, 4, 5, None),
    "SS": (1, None, 2, 3, 4, 5, 7),
    "CP": (1, 2, 3, 4, 5, 6, 8),
    "SO": (1, None, 3, 4, 5, 6, None),
}


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
    if not NUMBER.fullmatch(data) or sum(c.isdigit() for c in data) > 30:
        raise Unread("value", data)
    sign = -1 if data.startswith("-") else 1
    whole, _, frac = data.lstrip("+-").partition(".")
    frac = frac.ljust(4, "0")
    minutes, seconds, rest = int(frac[:2]), int(frac[2:4]), frac[4:]
    if minutes >= 60 or seconds >= 60:
        raise Unread("dms", data)
    secs = Fraction(int(whole or "0") * 3600 + minutes * 60 + seconds) + (Fraction(int(rest), 10 ** len(rest)) if rest else 0)
    return sign * secs / 3600


def angle(data, unit):
    if data == "":
        return None
    v = dms(data) if unit == "deg" else number(data)
    if abs(v) > FULL[unit]:
        raise Unread("turn", data)
    return v


def read(text):
    stations, problems = [], []
    distance = None  # "m", "none" (not read) or None (not given)
    unit = None  # "deg", "grad", "none" or None
    vertical = None  # "zenith", "horizon", "none" or None
    book_unit = None
    said = set()
    coords = {}
    station = None
    last_th = None
    for i, raw in enumerate(text.replace("\r\n", "\n").replace("\r", "\n").split("\n")):
        line = i + 1
        if not raw.strip():
            continue
        f = [x.strip() for x in raw.split(",")]
        get = lambda k: f[k] if k < len(f) else ""
        kind = f[0].upper()
        try:
            if kind == "CO":
                key, sep, value = ",".join(f[1:]).partition(":")
                key, value = key.strip().lower(), value.strip()
                if not sep:
                    continue
                if key == "dist units":
                    if value.lower().startswith("met"):
                        distance = "m"
                    else:
                        distance = "none"
                        problems.append({"line": line, "problem": TEXTS["distance"].format(line=line, data=value)})
                elif key == "angle units":
                    v = value.lower()
                    got = "deg" if v == "dddmmss" else "grad" if v in ("gon", "gons", "grad", "grads") else None
                    if got is None:
                        unit = "none"
                        problems.append({"line": line, "problem": TEXTS["angles"].format(line=line, data=value)})
                    elif book_unit is not None and got != book_unit:
                        unit = "none"
                        problems.append({"line": line, "problem": TEXTS["mixed"].format(line=line)})
                    else:
                        unit = book_unit = got
                elif key == "zero va":
                    v = value.lower()
                    if v in ("zenith", "horizon"):
                        vertical = v
                    else:
                        vertical = "none"
                        problems.append({"line": line, "problem": TEXTS["vertical"].format(line=line, data=value)})
            elif kind in COORDS:
                n, e, z = number(get(3)), number(get(4)), number(get(5))
                for name in (get(1), get(2)):
                    if name:
                        coords[name] = (e, n, z)
            elif kind == "ST":
                name = get(1) or get(2)
                hi = number(get(5))
                station = {"station": name}
                if hi is not None:
                    station["instrumentHeight"] = float(hi)
                if name in coords:
                    e, n, z = coords[name]
                    for key, v in (("east", e), ("north", n), ("height", z)):
                        if v is not None:
                            station[key] = float(v)
                station["observations"] = []
                station_back = get(3) or get(4)
                station["_back"] = station_back
                stations.append(station)
                last_th = None
            elif kind in OBS:
                if distance is None or unit is None:
                    if "before" not in said:
                        problems.append({"line": line, "problem": TEXTS["before"].format(line=line)})
                        said.add("before")
                    continue
                if distance == "none" or unit == "none" or vertical == "none":
                    continue
                t, alt, th_at, sd_at, ha_at, va_at, code_at = OBS[kind]
                hz = angle(get(ha_at), unit)
                va = angle(get(va_at), unit)
                slope = number(get(sd_at))
                th = number(get(th_at))
                if slope is not None and slope < 0:
                    raise Unread("negative", get(sd_at))
                if hz is not None and hz < 0:
                    hz += FULL[unit]
                zenith = va
                if va is not None and vertical == "horizon":
                    zenith = (QUARTER[unit] - va) % FULL[unit]
                if zenith is not None and zenith < 0:
                    raise Unread("below", get(va_at))
                target = get(t) or (get(alt) if alt else "")
                if not target and kind in ("F1", "F2") and station is not None:
                    target = station["_back"]
                if not target:
                    problems.append({"line": line, "problem": TEXTS["target"].format(line=line)})
                    continue
                if hz is None:
                    problems.append({"line": line, "problem": TEXTS["horizontal"].format(line=line, target=target)})
                    continue
                if vertical is None and va is not None and "zenith" not in said:
                    problems.append({"line": line, "problem": TEXTS["zenith"].format(line=line)})
                    said.add("zenith")
                if station is None:
                    station = {"station": "", "observations": [], "_back": ""}
                    stations.append(station)
                    last_th = None
                if th is not None:
                    last_th = th
                o = {"target": target, "hz": float(hz)}
                if zenith is not None:
                    o["zenith"] = float(zenith)
                if slope is not None:
                    o["slope"] = float(slope)
                if last_th is not None:
                    o["targetHeight"] = float(last_th)
                if code_at is not None and get(code_at):
                    o["code"] = get(code_at)
                o["line"] = line
                station["observations"].append(o)
        except Unread as e:
            problems.append({"line": line, "problem": TEXTS[e.key].format(line=line, data=e.data)})
    for s in stations:
        s.pop("_back", None)
    return {"unit": book_unit, "stations": stations, "problems": problems}


def cases():
    # Degrees (DDD.MMSS), zeniths: a known station (MP), a back sight on face one without its name (the station's back
    # sight), both faces, side shots with codes, a control point, a stakeout shot.
    dms_book = "\r\n".join(
        [
            "CO,Nikon RAW data format V2.00",
            "CO,KARNE",
            "CO,Dist Units: Metres",
            "CO,Angle Units: DDDMMSS",
            "CO,Zero azimuth: North",
            "CO,Zero VA: Zenith",
            "CO,Coord Order: NEZ",
            "MP,ST1,,4521800.000,412350.000,105.200,",
            "MP,P2,,4522045.000,412350.500,105.500,",
            "CO,Temp:20C Press:1013hPa Prism:0",
            "ST,ST1,,P2,,1.552,0.0000,0.0000",
            "F1,,1.700,,0.0004,,08:27:58",
            "F1,101,1.700,63.214,78.411304,91.065660,08:28:30",
            "F2,101,1.700,63.216,258.411952,268.531030,08:29:01",
            "SS,102,2.000,82.441,108.295820,90.234560,08:30:12,BINA",
            "CP,P9,,2.000,150.000,270.0000,90.0000,08:31:00,POL",
            "SO,1003,103,2.000,55.123,189.064800,88.494400,08:32:00,",
            "ST,ST2,,ST1,,1.480,0.0000,0.0000",
            "F1,ST1,2.000,83.392,0.0000,90.180036,08:40:00",
        ]
    ) + "\r\n"
    # Gon, from the horizon: elevation angles turned into zeniths; target heights carried; no Zero VA in the second.
    gon_book = "\n".join(
        [
            "CO,Dist Units: Metres",
            "CO,Angle Units: Gons",
            "CO,Zero VA: Horizon",
            "ST,S1,,,,1.500,0.0000,0.0000",
            "SS,A,1.600,10.000,50.0000,1.2345,10:00:00,",
            "F2,A,,10.002,250.0000,198.7655,10:00:30",
            "SS,B,,20.000,-10.0000,398.0000,10:01:00,",
        ]
    ) + "\n"
    # Units not known before an observation, units not read, numbers not read, no target, no horizontal reading,
    # no Zero VA (zeniths said once), an observation before any station.
    broken = "\n".join(
        [
            "SS,P0,1.6,10.0,10.0000,90.0000,09:00:00,",
            "CO,Dist Units: US-Feet",
            "CO,Angle Units: DDDMMSS",
            "SS,P1,1.6,10.0,10.0000,90.0000,09:00:00,",
            "CO,Dist Units: Metres",
            "SS,P2,1.6,10.0,10.0000,90.0000,09:00:00,",
            "SS,P3,1.6,10.0,10.6000,90.0000,09:00:00,",
            "SS,P4,1.6,10.0,10.0000,90.0060,09:00:00,",
            "SS,P5,1.6,10.0,360.0001,90.0000,09:00:00,",
            "SS,P6,1.6,-1.0,10.0000,90.0000,09:00:00,",
            "SS,P7,1.6,1e1,10.0000,90.0000,09:00:00,",
            "SS,,1.6,10.0,10.0000,90.0000,09:00:00,",
            "SS,P9,1.6,10.0,,90.0000,09:00:00,",
            "SS,P10,1.6,10.0,10.0000,-5.0000,09:00:00,",
            "CO,Angle Units: Gons",
            "SS,P11,1.6,10.0,10.0000,100.0000,09:00:00,",
            "CO,Angle Units: Mils",
            "SS,P12,1.6,10.0,10.0000,100.0000,09:00:00,",
        ]
    ) + "\n"
    return [
        ("derece (DDD.MMSS), başucu: bilinen istasyon, adsız geri bakış, iki durum, kodlar, kontrol ve aplikasyon ölçüsü", dms_book),
        ("gon, ufuktan düşey açı, taşınan prizma yüksekliği", gon_book),
        ("birimsiz ölçü, okunmayan birimler, bozuk sayılar, hedefsiz ve yatay açısız ölçü, istasyonsuz gözlem", broken),
    ]


def build():
    out = [{"name": name, "text": text, "expect": read(text)} for name, text in cases()]
    return {"format": "kentos.field-nikon", "version": 1, "source": "scripts/fixtures/field_nikon_cases.py (docs/adr/0169 §1; Nikon Nivo Series Instruction Manual, Nikon RAW V2.00)", "texts": TEXTS, "cases": out}


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
