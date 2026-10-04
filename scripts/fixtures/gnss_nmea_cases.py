#!/usr/bin/env python3
"""Independent reference of the NMEA 0183 GNSS log's reading (docs/adr/0169 §1, §6; step 6a).

Writes fixtures/gnss/v1/nmea.json from the NMEA 0183 standard's sentences alone (GGA: Global Positioning System Fix Data;
RMC: Recommended Minimum Specific GNSS Data), no KentOS code: a receiver's log read into its fixes, WGS 84 latitude and
longitude, heights, quality, satellites, HDOP and time.

The rules:

1. Lines end with LF, CR LF or CR; blank lines are passed over. A sentence begins at a line's first "$" (anything before
   it, a logger's time stamp, is passed over); a line without one is passed over.
2. A sentence is "$", a talker (two characters: GP, GN, GL, GA, GB, BD …), its type (three), fields separated by commas,
   then "*" and a checksum of two hexadecimal digits: the exclusive-or of every character between "$" and "*". A
   checksum that is not so or does not match leaves the sentence out, said. A sentence without "*" is read (old
   receivers wrote none).
3. GGA: 1 time (hhmmss with an optional fraction), 2 latitude (ddmm with a fraction of minutes), 3 N or S, 4 longitude
   (dddmm with a fraction), 5 E or W, 6 quality (0 no fix, 1 GPS, 2 DGPS, 3 PPS, 4 RTK fixed, 5 RTK float, 6 estimated,
   7 manual, 8 simulation), 7 satellites in use, 8 HDOP, 9 the antenna's height above mean sea level, 10 its unit M,
   11 the geoid's height above the WGS 84 ellipsoid (geoid separation), 12 its unit M. Quality 0 is no fix: counted and
   said once, at its first line. A latitude or longitude that is not so (minutes 60 or more, more than 90° or 180°, a
   hemisphere other than N, S, E, W) leaves the sentence out, said; so does a height in another unit than M, or a
   number that is not one. The fix's ellipsoidal height is the height above sea level plus the geoid separation when
   both are given; its height as written (above sea level) is kept beside it.
4. RMC: 9 its date (ddmmyy, the year 2000 + yy) is the date of the GGA fixes after it; a GGA's time is
   yyyy-mm-ddThh:mm:ss(.fraction)Z when a date is known, else hh:mm:ss(.fraction).
5. Other sentences (GSA, GSV, VTG, GLL, ZDA …) are passed over.
6. A latitude or longitude is its exact value (degrees plus minutes over sixty) rounded once to the nearest float64; a
   number is an optional sign, digits with an optional point, or a point and digits, at most 30 digits (so is a
   latitude or longitude); satellites are ASCII digits, at most 4294967295.
"""

import argparse
import json
import re
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "gnss" / "v1" / "nmea.json"
TEXTS = {
    "checksum": "Satır {line}: NMEA cümlesinin sağlama toplamı tutmuyor; cümle okunmadı.",
    "position": "Satır {line}: konum “{data}” okunmuyor; cümle okunmadı.",
    "value": "Satır {line}: “{data}” sayı değil; cümle okunmadı.",
    "unit": "Satır {line}: yükseklik birimi “{data}” okunmuyor; yalnız metre (M); cümle okunmadı.",
    "nofix": "Satır {line}: konumu olmayan GGA cümleleri ({count} cümle, kalite 0) okunmadı.",
}
NUMBER = re.compile(r"[+-]?([0-9]+(\.[0-9]*)?|\.[0-9]+)")
QUALITY = {"1": "GPS", "2": "DGPS", "3": "PPS", "4": "RTK sabit", "5": "RTK kayan", "6": "tahmini", "7": "elle", "8": "benzetim"}


class Unread(Exception):
    def __init__(self, key, data):
        super().__init__(key)
        self.key, self.data = key, data


def digits(text):
    return sum(c.isdigit() for c in text)


def number(data):
    if data == "":
        return None
    if not NUMBER.fullmatch(data) or digits(data) > 30:
        raise Unread("value", data)
    return Fraction(data)


def coordinate(value, hemisphere, degrees_digits, limit, positive, negative):
    """ddmm.mmmm (or dddmm.mmmm) and its hemisphere: the exact degrees."""
    text = f"{value},{hemisphere}"
    m = re.fullmatch(r"([0-9]+)(\.[0-9]*)?", value)
    if not m or len(m.group(1)) != degrees_digits + 2 or hemisphere not in (positive, negative) or digits(value) > 30:
        raise Unread("position", text)
    whole = m.group(1)
    degrees = int(whole[:degrees_digits])
    minutes = Fraction(whole[degrees_digits:] + (m.group(2) or ""))
    if minutes >= 60:
        raise Unread("position", text)
    v = degrees + minutes / 60
    if v > limit:
        raise Unread("position", text)
    return -v if hemisphere == negative else v


def clock(t):
    m = re.fullmatch(r"([0-9]{2})([0-9]{2})([0-9]{2})(\.[0-9]+)?", t)
    if not m or int(m.group(1)) > 23 or int(m.group(2)) > 59 or int(m.group(3)) > 60:
        return None
    return f"{m.group(1)}:{m.group(2)}:{m.group(3)}{m.group(4) or ''}"


def read(text):
    points, problems = [], []
    date = None
    nofix = None
    for i, raw in enumerate(text.replace("\r\n", "\n").replace("\r", "\n").split("\n")):
        line = i + 1
        at = raw.find("$")
        if at < 0:
            continue
        s = raw[at:].strip()
        body, star, check = s[1:].partition("*")
        if star:
            x = 0
            for b in body.encode("utf-8"):
                x ^= b
            if not re.fullmatch(r"[0-9A-Fa-f]{2}", check) or int(check, 16) != x:
                problems.append({"line": line, "problem": TEXTS["checksum"].format(line=line)})
                continue
        f = body.split(",")
        kind = f[0][2:] if len(f[0]) == 5 else ""
        get = lambda k: f[k].strip() if k < len(f) else ""
        try:
            if kind == "RMC":
                d = get(9)
                m = re.fullmatch(r"([0-9]{2})([0-9]{2})([0-9]{2})", d)
                if m and 1 <= int(m.group(2)) <= 12 and 1 <= int(m.group(1)) <= 31:
                    date = f"20{m.group(3)}-{m.group(2)}-{m.group(1)}"
            elif kind == "GGA":
                quality = get(6)
                if quality in ("", "0"):
                    if nofix is None:
                        nofix = [line, 0]
                    nofix[1] += 1
                    continue
                lat = coordinate(get(2), get(3), 2, 90, "N", "S")
                lon = coordinate(get(4), get(5), 3, 180, "E", "W")
                sats = get(7)
                if sats and not (sats.isascii() and sats.isdigit() and int(sats) <= 0xFFFFFFFF):
                    raise Unread("value", sats)
                hdop = number(get(8))
                alt = number(get(9))
                if alt is not None and get(10) not in ("M", ""):
                    raise Unread("unit", get(10))
                sep = number(get(11))
                if sep is not None and get(12) not in ("M", ""):
                    raise Unread("unit", get(12))
                p = {"kind": "gga", "lat": float(lat), "lon": float(lon)}
                if alt is not None:
                    p["height"] = float(alt)
                    if sep is not None:
                        p["ellipsoidal"] = float(alt + sep)
                if sep is not None:
                    p["geoid"] = float(sep)
                t = clock(get(1))
                if t is not None:
                    p["time"] = f"{date}T{t}Z" if date else t
                p["fix"] = QUALITY.get(quality, quality)
                if sats:
                    p["satellites"] = int(sats)
                if hdop is not None:
                    p["hdop"] = float(hdop)
                p["line"] = line
                points.append(p)
        except Unread as e:
            problems.append({"line": line, "problem": TEXTS[e.key].format(line=line, data=e.data)})
    if nofix:
        problems.append({"line": nofix[0], "problem": TEXTS["nofix"].format(line=nofix[0], count=nofix[1])})
        problems.sort(key=lambda p: p["line"])
    return {"points": points, "problems": problems}


def sentence(body):
    x = 0
    for c in body:
        x ^= ord(c)
    return f"${body}*{x:02X}"


def cases():
    log = "\r\n".join(
        [
            sentence("GPRMC,101500.00,A,4045.2345,N,02923.1234,E,0.02,0.0,041026,,,D"),
            sentence("GPGGA,101500.00,4045.23450,N,02923.12340,E,4,14,0.6,105.234,M,36.123,M,1.0,0001"),
            "2026-10-04 10:15:01 " + sentence("GNGGA,101501.00,4045.23455,N,02923.12345,E,5,12,0.9,105.240,M,36.123,M,,"),
            sentence("GPGSV,3,1,11,03,03,111,00,04,15,270,00,06,01,010,00,13,06,292,00"),
            sentence("GPGGA,101502,0000.0000,S,00000.0000,W,1,05,2.5,-0.5,M,,M,,"),
            sentence("GPGGA,101503.5,8959.99999,N,17959.99999,W,2,08,1.2,,,,,,"),
        ]
    ) + "\r\n"
    broken = "\n".join(
        [
            sentence("GPGGA,101500.00,4045.2345,N,02923.1234,E,0,00,99.9,,,,,,"),
            sentence("GPGGA,101501.00,4045.2345,N,02923.1234,E,0,00,99.9,,,,,,"),
            "$GPGGA,101502.00,4045.2345,N,02923.1234,E,1,08,1.0,100.0,M,36.0,M,,*00",
            "$GPGGA,101503.00,4045.2345,N,02923.1234,E,1,08,1.0,100.0,M,36.0,M,,*G1",
            "$GPGGA,101504.00,4045.2345,N,02923.1234,E,1,08,1.0,100.0,M,36.0,M,,",
            sentence("GPGGA,101505.00,4060.0000,N,02923.1234,E,1,08,1.0,100.0,M,36.0,M,,"),
            sentence("GPGGA,101506.00,9100.0000,N,02923.1234,E,1,08,1.0,100.0,M,36.0,M,,"),
            sentence("GPGGA,101507.00,4045.2345,X,02923.1234,E,1,08,1.0,100.0,M,36.0,M,,"),
            sentence("GPGGA,101508.00,445.2345,N,02923.1234,E,1,08,1.0,100.0,M,36.0,M,,"),
            sentence("GPGGA,101509.00,4045.2345,N,02923.1234,E,1,08,1.0,100.0,F,36.0,M,,"),
            sentence("GPGGA,101510.00,4045.2345,N,02923.1234,E,1,x8,1.0,100.0,M,36.0,M,,"),
            sentence("GPGGA,101511.00,4045.2345,N,02923.1234,E,1,08,1.0a,100.0,M,36.0,M,,"),
            sentence("GPGGA,99,4045.2345,N,02923.1234,E,1,08,1.0,100.0,M,36.0,M,,"),
            "no sentence here",
            sentence("GPRMC,101512,A,4045.2345,N,02923.1234,E,,,311326,,"),
            sentence("GPGGA,101512.00,4045.2345,N,02923.1234,E,1,08,1.0,100.0,M,36.0,M,,"),
        ]
    ) + "\n"
    return [
        ("RTK sabit ve kayan, RMC tarihi, kayıt öneki, GSV, güney ve batı, sınırda konum", log),
        ("kalite 0, sağlama toplamı, bozuk konum, birim ve sayılar, tarihsiz zaman", broken),
    ]


def build():
    out = [{"name": name, "text": text, "expect": read(text)} for name, text in cases()]
    return {"format": "kentos.gnss-nmea", "version": 1, "source": "scripts/fixtures/gnss_nmea_cases.py (docs/adr/0169 §1; NMEA 0183 GGA, RMC)", "texts": TEXTS, "cases": out}


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
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
