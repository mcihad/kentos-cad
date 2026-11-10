#!/usr/bin/env python3
"""The shared cases of `map.survey.query` and `map.survey.edit` (plan/map-cad.md MAP-03, docs/adr/1340), written from
the ADR without KentOS code, and the GNSS files they read (fixtures/map-survey/v1).

    python3 scripts/fixtures/map_survey_cases.py           # writes fixtures/commands/v1/map.survey.*.json and the files
    python3 scripts/fixtures/map_survey_cases.py --check   # writes nothing; compares

Needs pyproj (PROJ) and the grids of fixtures/geoid/v1, as crs_chain_cases.py: without pyproj here the script runs
itself again under QGIS's Python on macOS.

The drawing is a map project (TUREF / TM30, ellipsoidal heights) with a campaign of three sessions: mark N1 measured on
two days (a RINEX 3.05 and a RINEX 2.11 header, two antenna heights), mark N2 by an RTK receiver's NMEA log (a float
fall, a time jump, a cut sentence, a position without a height). N1's point is accepted as the mean of two processed
solutions; a surface line rests on it. Every rule, its order and its sentence is ADR 1340's, written here by hand:

- RINEX: the label in columns 61–80; the version (1–9), the type (21, `O`), MARKER NAME (1–60), MARKER NUMBER (1–20),
  REC # / TYPE / VERS (1–20, 21–40, 41–60), ANT # / TYPE (1–20, 21–40), ANTENNA: DELTA H/E/N (1–14, 15–28, 29–42; E and
  N kept when not 0), APPROX POSITION XYZ, INTERVAL (1–10), TIME OF FIRST / LAST OBS (5 × 6 digits, seconds 31–43, the
  time system 49–51); the header ends at END OF HEADER.
- NMEA and GPX: gnss_nmea_cases.py's reader (the standard's sentences), independent of KentOS as this script.
- The ARP's vertical height: measured; slant √(S² − R²) − offset. A mark's height h + applied − H.
- A receiver solution: the positions from..to whose fix is given, summed left to right (not Python's `sum`), divided.
- An accepted point: single, mean (left to right) or weighted (1/σ² per component, σ = √(1/Σw)); into the project by
  the coordinate chain (crs_vertical_cases.py and geoid_cases.py: PROJ, crs_chain_rules.py's verdict).
- Last measured: the latest session end (else start); a tie to the larger id number.
- Replay: fixes in the order first seen; RTK sabit followed by another fix; steps between timed positions in
  milliseconds (⌊1000·Δ + 0.5⌋), the usual one the most frequent positive (a tie to the smaller), a jump ≤ 0 or > 2×.
- ADR 4120, `places`: each known point's own place, carried as an accepted point is: the epoch decision of the campaign
  of the first baseline on its mark (else the project's); at another epoch first by its own velocity, linearly in its
  frame's Earth-centred X, Y, Z (PROJ: TUREF 3D → geocentric and back), else along that campaign's velocity grid by
  PROJ, the shift east, north, up at its given place; then the chain. Its mark's accepted point and place − point.
"""

import copy
import hashlib
import json
import math
import os
import shutil
import sys
import tempfile
from pathlib import Path

try:
    import pyproj  # noqa: F401
except ImportError:
    qgis = sorted(Path("/Applications").glob("QGIS*.app/Contents"))
    if not qgis or os.environ.get("KENTOS_QGIS_PYTHON"):
        raise
    res = qgis[-1] / "Resources"
    env = dict(os.environ, KENTOS_QGIS_PYTHON="1", PYTHONHOME=str(res),
               PYTHONPATH=os.pathsep.join(str(res / p) for p in ("python3.12", "python3.12/site-packages",
                                                                 "python3.12/lib-dynload")))
    exe = qgis[-1] / "MacOS" / "python3.12"
    os.execve(str(exe), [str(exe), *sys.argv], env)

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_vertical_cases as cv  # noqa: E402
import geoid_cases as gc  # noqa: E402
import gnss_nmea_cases as nm  # noqa: E402
import pyproj  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
FILES = ROOT / "fixtures/map-survey/v1"
ELL = {"kind": "ellipsoidal"}
# ADR 4120: a geographic frame's 3D system and its Earth-centred one (PROJ), for a known point's own velocity.
GEO3D = {5252: (5251, 5250), 4326: (4979, 4978)}


def geocentric(srid, lat, lon, h):
    x, y, z = pyproj.Transformer.from_crs(GEO3D[srid][0], GEO3D[srid][1], always_xy=True).transform(lon, lat, h)
    return [x, y, z]


def geographic(srid, p):
    lon, lat, h = pyproj.Transformer.from_crs(GEO3D[srid][1], GEO3D[srid][0], always_xy=True).transform(*p)
    return lat, lon, h


def enu_rows(lat, lon):
    la, lo = math.radians(lat), math.radians(lon)
    return [[-math.sin(lo), math.cos(lo), 0.0],
            [-math.sin(la) * math.cos(lo), -math.sin(la) * math.sin(lo), math.cos(la)],
            [math.cos(la) * math.cos(lo), math.cos(la) * math.sin(lo), math.sin(la)]]


def epoch_words(e):
    """An epoch as a sentence writes it: two decimals, a decimal comma (2026,84)."""
    return f"{e:.2f}".replace(".", ",")
HIZ = gc.model("hiz.tif", "Hız alanı (sentetik)")

# ── The files ────────────────────────────────────────────────────────────────────────────────────────────────────


def rx(body, label):
    return f"{body:<60}{label:<20}\n"


def obs_time(y, mo, d, h, mi, sec, system):
    return f"{y:6d}{mo:6d}{d:6d}{h:6d}{mi:6d}{sec:13.7f}     {system:<3}"


def rinex(version, kind_text, marker, number, rec, ant, xyz, dh, de, dn, interval, first, last):
    out = rx(f"{version:9.2f}           {kind_text}", "RINEX VERSION / TYPE")
    out += rx(marker, "MARKER NAME")
    if number:
        out += rx(number, "MARKER NUMBER")
    out += rx(f"{'KENTOS':<20}{'OFIS':<40}", "OBSERVER / AGENCY")
    out += rx(f"{rec[0]:<20}{rec[1]:<20}{rec[2]:<20}", "REC # / TYPE / VERS")
    out += rx(f"{ant[0]:<20}{ant[1]:<20}", "ANT # / TYPE")
    out += rx(f"{xyz[0]:14.4f}{xyz[1]:14.4f}{xyz[2]:14.4f}", "APPROX POSITION XYZ")
    out += rx(f"{dh:14.4f}{de:14.4f}{dn:14.4f}", "ANTENNA: DELTA H/E/N")
    if interval:
        out += rx(f"{interval:10.3f}", "INTERVAL")
    out += rx(obs_time(*first), "TIME OF FIRST OBS")
    if last:
        out += rx(obs_time(*last), "TIME OF LAST OBS")
    out += rx("", "END OF HEADER")
    out += "> 2026 11 03 09 30  0.0000000  0 12\n"
    return out.encode()


GUN1 = rinex(3.05, "OBSERVATION DATA    M", "N1", "N1-2026", ("5123K40001", "TRIMBLE R10", "5.52"),
             ("1441112222", "TRM57971.00     NONE"), (4194423.0, 2466101.5, 4055712.25), 1.532, 0.0, 0.0, 30.0,
             (2026, 11, 3, 9, 30, 0.0, "GPS"), (2026, 11, 3, 10, 30, 0.0, "GPS"))
GUN2 = rinex(2.11, "OBSERVATION DATA    G (GPS)", "N1", "", ("1830456", "LEICA GS18", "8.10"),
             ("1830456", "LEIGS18         NONE"), (4194423.0, 2466101.5, 4055712.25), 1.61, 0.0, 0.0, 15.0,
             (2026, 11, 4, 8, 0, 0.0, "GPS"), (2026, 11, 4, 9, 0, 0.0, "GPS"))
GUN3 = rinex(4.00, "OBSERVATION DATA    M", "N3", "", ("5123K40001", "TRIMBLE R10", "5.52"),
             ("1441112222", "TRM57971.00     NONE"), (4194500.0, 2466000.0, 4055700.0), 1.2, 0.1, 0.0, 0.0,
             (2026, 11, 5, 7, 15, 30.5, "GPS"), None)
NAV = rx(f"{3.05:9.2f}           NAVIGATION DATA     G", "RINEX VERSION / TYPE").encode()
CUT = (rx(f"{3.05:9.2f}           OBSERVATION DATA    M", "RINEX VERSION / TYPE") + rx("N1", "MARKER NAME")).encode()


def gga(t, lat, lon, q, sats, hdop, alt, sep):
    body = f"GPGGA,{t},{lat},N,{lon},E,{q},{sats},{hdop},{alt},M,{sep},M,1.0,0001"
    return nm.sentence(body)


N2_LOG = "\n".join([
    nm.sentence("GPRMC,093000.00,A,3946.8060000,N,03030.9000000,E,0.02,0.0,031126,,,R"),
    gga("093000.00", "3946.8060000", "03030.9000000", 4, 14, "0.62", "850.123", "36.500"),
    gga("093001.00", "3946.8060100", "03030.9000200", 4, 14, "0.63", "850.131", "36.500"),
    gga("093002.00", "3946.8060050", "03030.9000100", 5, 12, "0.90", "850.170", "36.500"),
    gga("093003.00", "3946.8060000", "03030.9000000", 4, 13, "0.70", "850.127", "36.500"),
    "$GPGGA,093004.00,3946.8060000,N,03030.9000000,E,4,13,0.70,850.12",
    gga("093010.00", "3946.8059900", "03030.8999900", 4, 13, "0.71", "850.119", "36.500"),
    gga("093011.00", "3946.8060000", "03030.9000000", 4, 12, "0.72", "", ""),
]) + "\n"
# The cut sentence has no checksum: an old receiver's; it is read unless its fields do not hold: its height's unit is
# missing — a position read without a unit. Make it a checksum failure instead (a cut line ends where it ends).
N2_LOG = N2_LOG.replace("$GPGGA,093004.00,3946.8060000,N,03030.9000000,E,4,13,0.70,850.12\n",
                        "$GPGGA,093004.00,3946.8060000,N,03030.9000000,E,4,13,0.70,850.12*00\n")
EMPTY_LOG = nm.sentence("GPRMC,093000.00,A,3946.8060000,N,03030.9000000,E,0.02,0.0,031126,,,R") + "\n"
REPORT = ("Baz işleme raporu (örnek)\nNokta: N1\nYazılım: TBC 5.90\nEfemeris: kesin (IGS)\nYükseklik maskesi: 10°\n"
          "Anten modeli: IGS20 ANTEX\n").encode()
WRITTEN = {"n1-gun1.26o": GUN1, "n1-gun2.26o": GUN2, "n3-gun3.26o": GUN3, "nav.26n": NAV, "kesik.26o": CUT,
           "n2.nmea": N2_LOG.encode(), "bos.nmea": EMPTY_LOG.encode(), "rapor-n1.txt": REPORT}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def fx(name):
    return f"$fixture:map-survey/v1/{name}"


# ── The readers (ADR 1340 §4) ────────────────────────────────────────────────────────────────────────────────────


def survey_time(t):
    """YYYY-MM-DDThh:mm:ss[.f]Z as a sortable tuple, or None."""
    if len(t) < 20 or t[4] != "-" or t[7] != "-" or t[10] != "T" or t[13] != ":" or t[16] != ":" or not t.endswith("Z"):
        return None
    body = t[:-1]

    def num(s):
        return int(s) if s.isascii() and s.isdigit() else None
    parts = [num(body[0:4]), num(body[5:7]), num(body[8:10]), num(body[11:13]), num(body[14:16]), num(body[17:19])]
    if None in parts:
        return None
    rest = body[19:]
    if rest == "":
        frac = ""
    else:
        if not rest.startswith(".") or not (1 <= len(rest) - 1 <= 9) or not rest[1:].isdigit():
            return None
        frac = rest[1:].rstrip("0")
    y, mo, d, h, mi, s = parts
    leap = (y % 4 == 0 and y % 100 != 0) or y % 400 == 0
    days = [31, 29 if leap else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    if not (1900 <= y <= 2100) or not (1 <= mo <= 12) or not (1 <= d <= days[mo - 1]) or h > 23 or mi > 59 or s > 60:
        return None
    return (y, mo, d, h, mi, s, frac)


def seconds(t):
    p = survey_time(t)
    if p is None:
        return None
    y, mo, d, h, mi, s, frac = p
    days = 0
    for year in range(1900, y):
        days += 366 if ((year % 4 == 0 and year % 100 != 0) or year % 400 == 0) else 365
    leap = (y % 4 == 0 and y % 100 != 0) or y % 400 == 0
    months = [31, 29 if leap else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    days += sum(months[:mo - 1]) + d - 1
    whole = days * 86400 + h * 3600 + mi * 60 + s
    return float(whole) + (float("0." + frac) if frac else 0.0)


def cols(line, a, b):
    return line[a:b].strip()


def read_rinex(data):
    lines = [l.rstrip("\r") for l in data.decode().split("\n")]
    if cols(lines[0], 60, 80) != "RINEX VERSION / TYPE":
        return None, "Dosya RINEX değil: ilk satırda “RINEX VERSION / TYPE” yok."
    kind = cols(lines[0], 20, 21)
    if kind != "O":
        return None, f"RINEX dosyası gözlem dosyası değil (tür “{kind}”); oturum gözlem dosyasından okunur."
    h = {"facts": {"version": cols(lines[0], 0, 9)}}
    ended = False
    for line in lines[1:]:
        label = cols(line, 60, 80)
        if label == "END OF HEADER":
            ended = True
            break
        if label == "MARKER NAME" and cols(line, 0, 60):
            h["marker"] = cols(line, 0, 60)
        elif label == "MARKER NUMBER" and cols(line, 0, 20):
            h["facts"]["markerNumber"] = cols(line, 0, 20)
        elif label == "REC # / TYPE / VERS" and cols(line, 20, 40):
            r = {"type": cols(line, 20, 40)}
            for k, a, b in (("serial", 0, 20), ("firmware", 40, 60)):
                if cols(line, a, b):
                    r[k] = cols(line, a, b)
            h["receiver"] = r
        elif label == "ANT # / TYPE":
            a = h.setdefault("antenna", {"height": 0.0})
            for k, x, y in (("serial", 0, 20), ("type", 20, 40)):
                if cols(line, x, y):
                    a[k] = cols(line, x, y)
        elif label == "ANTENNA: DELTA H/E/N":
            a = h.setdefault("antenna", {"height": 0.0})
            a["height"] = float(cols(line, 0, 14))
            for k, x, y in (("east", 14, 28), ("north", 28, 42)):
                v = float(cols(line, x, y))
                if v != 0.0:
                    a[k] = v
        elif label == "APPROX POSITION XYZ":
            h["facts"]["approximate"] = [float(cols(line, 0, 14)), float(cols(line, 14, 28)), float(cols(line, 28, 42))]
        elif label == "INTERVAL":
            h["facts"]["interval"] = float(cols(line, 0, 10))
        elif label in ("TIME OF FIRST OBS", "TIME OF LAST OBS"):
            y, mo, d, hh, mi = (int(cols(line, k, k + 6)) for k in (0, 6, 12, 18, 24))
            sec = cols(line, 30, 43)
            whole, _, frac = sec.partition(".")
            text = f"{y:04d}-{mo:02d}-{d:02d}T{hh:02d}:{mi:02d}:{whole.zfill(2)}{'.' + frac if frac else ''}Z"
            assert survey_time(text)
            if label == "TIME OF FIRST OBS":
                h["start"] = text
                system = cols(line, 48, 51)
                if system in ("GPS", "UTC", "GLO", "GAL", "BDT", "QZS", "IRN"):
                    h["timeSystem"] = system.lower()
            else:
                h["end"] = text
    if not ended:
        return None, "RINEX başlığının sonu (END OF HEADER) yok; dosya kesilmiş olabilir."
    return h, None


# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────


def uid(n):
    return f"0192f5c0-1340-7000-8000-{n:012x}"


P1, S1, F1, K1 = (uid(n) for n in range(1, 5))
GHOST = uid(0x999)


def layer(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def campaign_name():
    return "GNSS ölçümü 2026-11"


def antenna_vertical(a):
    if a.get("reference", "arp") == "arp":
        return a["height"]
    r = a["radius"]
    return math.sqrt(a["height"] * a["height"] - r * r) - a.get("offset", 0.0)


def session_from_rinex(id_, name, mark=None):
    h, err = read_rinex(WRITTEN[name])
    assert err is None
    s = {"id": id_, "campaign": "kp-1", "mark": mark or h["marker"]}
    for k in ("start", "end", "timeSystem", "receiver", "antenna"):
        if k in h:
            s[k] = copy.deepcopy(h[k])
    s["file"] = {"path": fx(name), "revision": sha(WRITTEN[name]), "format": "rinex"}
    s["rinex"] = h["facts"]
    return s


def nmea_points(name):
    return nm.read(WRITTEN[name].decode())


def session_from_nmea(id_, name, mark, antenna=None):
    read = nmea_points(name)
    pts = read["points"]
    s = {"id": id_, "campaign": "kp-1", "mark": mark}
    times = [p["time"] for p in pts if "time" in p and survey_time(p["time"])]
    if times:
        s["start"], s["end"], s["timeSystem"] = times[0], times[-1], "utc"
    if antenna:
        s["antenna"] = antenna
    s["file"] = {"path": fx(name), "revision": sha(WRITTEN[name]), "format": "nmea"}
    s["observations"] = pts
    return s


def receiver_solution(id_, session, srid, applied, frm, to, fixes, epoch=None):
    pts = session["observations"][frm:to + 1]
    taken = [p for p in pts if (not fixes or p.get("fix") in fixes) and "ellipsoidal" in p]
    lat = lon = h = 0.0
    for p in taken:
        lat += p["lat"]
        lon += p["lon"]
        h += p["ellipsoidal"]
    k = float(len(taken))
    s = {"id": id_, "session": session["id"], "kind": "receiver", "lat": lat / k, "lon": lon / k, "height": h / k,
         "srid": srid, "antennaApplied": applied}
    if epoch is not None:
        s["epoch"] = epoch
    if all(p.get("fix") == taken[0].get("fix") for p in taken) and "fix" in taken[0]:
        s["fix"] = taken[0]["fix"]
    s["epochs"] = len(taken)
    sats = [p["satellites"] for p in taken if "satellites" in p]
    if sats:
        s["satellites"] = min(sats)
    hd = [p["hdop"] for p in taken if "hdop" in p]
    if hd:
        m = None
        for v in hd:
            m = v if m is None or v > m else m
        s["hdop"] = m
    s["from"], s["to"] = frm, to
    if fixes:
        s["fixes"] = list(fixes)
    return s


CZ1 = {"id": "cz-1", "session": "os-1", "kind": "processed", "lat": 39.7767123, "lon": 30.5206456, "height": 845.231,
       "srid": 5252, "epoch": 2005.0, "antennaApplied": 1.532, "fix": "Sabit (L1/L2)", "epochs": 120,
       "sigma": [0.004, 0.003, 0.009], "covariance": [1.6e-05, 1e-06, 2e-06, 9e-06, 1e-06, 8.1e-05],
       "correction": "TUSAGA-Aktif (KNY1, AFYN)",
       "report": {"path": fx("rapor-n1.txt"), "revision": sha(REPORT), "format": "report"},
       "recipe": "TBC 5.90; kesin efemeris (IGS); 10° maske; IGS20 ANTEX; anten 1.532 m ARP", "adjust": True}
CZ2 = {"id": "cz-2", "session": "os-2", "kind": "processed", "lat": 39.7767124, "lon": 30.5206455, "height": 846.847,
       "srid": 5252, "epoch": 2005.0, "antennaApplied": 0.0, "fix": "Sabit (L1/L2)", "epochs": 240,
       "sigma": [0.005, 0.004, 0.012], "adjust": True}
CZ4 = {"id": "cz-4", "session": "os-1", "kind": "ppp", "lat": 39.7767131, "lon": 30.5206441, "height": 845.412,
       "srid": 5252, "epoch": 2005.0, "antennaApplied": 1.532, "sigma": [0.02, 0.02, 0.05],
       "correction": "CSRS-PPP", "excluded": "Çok yollu yansıma şüphesi; 40 dakikalık veri"}
N2_ANTENNA = {"type": "Jalon 2 m", "height": 2.0}


def start_survey():
    s1 = session_from_rinex("os-1", "n1-gun1.26o")
    s2 = session_from_rinex("os-2", "n1-gun2.26o")
    s3 = session_from_nmea("os-3", "n2.nmea", "N2", N2_ANTENNA)
    cz3 = receiver_solution("cz-3", s3, 4326, 0.0, 0, 4, ["RTK sabit"])
    survey = {
        "campaigns": [{"id": "kp-1", "name": campaign_name(), "work": "is-1", "epoch": 2005.0}],
        "sessions": [s1, s2, s3],
        "solutions": [copy.deepcopy(CZ1), copy.deepcopy(CZ2), cz3, copy.deepcopy(CZ4)],
        "points": [],
        "uses": [],
    }
    return survey


def setup(work="map", survey="start", locked_gnss=False, heights=True):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work}
    if heights:
        settings["heights"] = ELL
    if work == "map":
        settings["mapWork"] = {"items": [{"id": "is-1", "kind": "campaign", "name": campaign_name(),
                                          "layers": ["gnss"]}]}
    d = {"format": "kentos.document", "version": 2, "name": "GNSS arşivi", "settings": settings,
         "origin": {"x": ORIGIN[0], "y": ORIGIN[1]},
         "layers": [layer("0", "0"), layer("gnss", "GNSS noktaları", locked=locked_gnss), layer("yuzey", "Yüzey"),
                    layer("cizim", "Çizim"), layer("kilitli", "Kilitli", locked=True)],
         "activeLayer": "gnss", "entities": [], "uids": [], "styles": {"items": [], "categories": []}}
    model = Model(d)
    if survey == "start":
        model.survey = start_survey()
        r = model.compute("N1", ["cz-1", "cz-2"], "mean")
        model.survey["points"].append({"id": "kn-1", "mark": "N1", "uid": P1, "result": r})
        model.survey["uses"].append({"id": "kb-1", "operation": "Yüzey üret", "uids": [S1],
                                     "points": [{"point": "kn-1", "revision": 1}]})
        model.entities[1] = point_entity("gnss", r, "N1", "kn-1")
        model.uids[1] = P1
    else:
        # A drawing without an archive keeps its first object too: a file's objects are numbered from 1 (the KCAD v2
        # writer's rule, which the Python runner opens the drawing with).
        model.entities[1] = {"kind": "point", "id": 1, "layerId": "gnss", "attrs": {"Ad": "N1"},
                             "p": {"x": ORIGIN[0] + 1.0, "y": ORIGIN[1] + 1.0}}
        model.uids[1] = P1
    model.entities[2] = {"kind": "line", "id": 2, "layerId": "yuzey", "attrs": {},
                         "a": {"x": ORIGIN[0], "y": ORIGIN[1]}, "b": {"x": ORIGIN[0] + 30.0, "y": ORIGIN[1] + 10.0}}
    model.uids[2] = S1
    model.entities[3] = {"kind": "line", "id": 3, "layerId": "cizim", "attrs": {},
                         "a": {"x": ORIGIN[0], "y": ORIGIN[1] + 20.0}, "b": {"x": ORIGIN[0] + 10.0, "y": ORIGIN[1] + 25.0}}
    model.uids[3] = F1
    model.entities[4] = {"kind": "point", "id": 4, "layerId": "kilitli", "attrs": {"Ad": "K1"},
                         "p": {"x": ORIGIN[0] + 5.0, "y": ORIGIN[1] + 5.0}}
    model.uids[4] = K1
    return model


def point_entity(layer_id, r, mark, pid):
    e = {"kind": "point", "id": 1, "layerId": layer_id,
         "attrs": {"Ad": mark, "Kabul": pid, "Revizyon": str(r["revision"]), "Yöntem": METHOD[r["method"]],
                   "Çözüm": r.get("adjustment") or ", ".join(r["solutions"]), "Arazi ölçümü": campaign_name()},
         "p": {"x": r["x"], "y": r["y"]}}
    if "z" in r:
        e["z"] = r["z"]
    return e


METHOD = {"single": "Tek çözüm", "mean": "Ortalama", "weighted": "Ağırlıklı ortalama", "network": "Ağ dengelemesi"}


def m3(v):
    return f"{v:.3f}"


def epoch_text(e):
    return "yok" if e is None else f"{e:.4f}"


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(code)
        self.code, self.message, self.path = code, message, path


class Model:
    """The drawing's objects (slot → entity, slot → uid), its settings and the archive."""

    def __init__(self, d):
        self.d = d
        self.entities, self.uids = {}, {}
        self.survey = None

    def clone(self):
        m = Model(copy.deepcopy(self.d))
        m.entities, m.uids, m.survey = copy.deepcopy(self.entities), dict(self.uids), copy.deepcopy(self.survey)
        return m

    def document(self):
        d = copy.deepcopy(self.d)
        slots = sorted(self.entities)
        d["entities"] = [copy.deepcopy(self.entities[s]) for s in slots]
        d["uids"] = [self.uids[s] for s in slots]
        if self.survey and any(self.survey.values()):
            d["settings"]["mapSurvey"] = written(self.survey)
        return d

    # The archive's records.
    def get(self, kind, id_):
        return next((x for x in self.survey[kind] if x["id"] == id_), None)

    def mark_of(self, sol):
        s = self.get("sessions", sol["session"])
        return s["mark"] if s else None

    def campaign_of(self, sol):
        s = self.get("sessions", sol["session"])
        return self.get("campaigns", s["campaign"]) if s else None

    def decision(self, campaign):
        if campaign and campaign.get("epoch") is not None:
            return campaign["epoch"]
        return self.d["settings"].get("coordinateEpoch")

    def point_using(self, sid):
        return next((p for p in self.survey["points"] if sid in p["result"]["solutions"]), None)

    def present(self, u):
        return u in self.uids.values()

    def vertical(self, session):
        a = session.get("antenna")
        return antenna_vertical(a) if a else None

    # §3: an accepted point.
    def compute(self, mark, ids, method, at="operation.solutions"):
        sols = []
        for i, sid in enumerate(ids):
            s = self.get("solutions", sid)
            if s is None:
                raise Refused("unknown_survey_record", f"GNSS arşivinde “{sid}” kimlikli çözüm yok.", f"{at}[{i}]")
            if sid in ids[:i]:
                raise Refused("invalid_survey_value", f"“{sid}” çözümü iki kez verilmiş.", f"{at}[{i}]")
            sols.append(s)
        for i, s in enumerate(sols):
            if "excluded" in s:
                raise Refused("solution_excluded", f"“{s['id']}” çözümü dışlanmış ({s['excluded']}); önce dışlamayı "
                                                   "kaldırın ya da başka çözüm seçin.", f"{at}[{i}]")
            if self.mark_of(s) != mark:
                raise Refused("mark_mismatch", f"“{s['id']}” çözümü “{self.mark_of(s)}” işaretinin, “{mark}” "
                                               "işaretinin değil.", f"{at}[{i}]")
        n = len(sols)
        if not (n == 1 if method == "single" else n >= 2):
            need = "tek çözüm" if method == "single" else "en az iki çözüm"
            raise Refused("method_count", f"{METHOD[method]} yöntemi {need} ister; {n} çözüm verildi.", at)
        if method == "weighted":
            for i, s in enumerate(sols):
                if "sigma" not in s:
                    raise Refused("sigma_missing", "Ağırlıklı ortalama her çözümün σ'sını ister; "
                                                   f"“{s['id']}” çözümünün σ'sı yok.", f"{at}[{i}]")
        first = sols[0]
        camp = self.campaign_of(first)
        for i, s in enumerate(sols[1:], 1):
            c = self.campaign_of(s)
            same_frame = s["srid"] == first["srid"] and s.get("epoch") == first.get("epoch")
            key = lambda c: (c.get("epoch"), c.get("velocity")) if c else None  # noqa: E731
            if not same_frame or key(c) != key(camp):
                raise Refused("solutions_differ", f"“{first['id']}” ile “{s['id']}” aynı çerçeve ve epokta değil ya "
                                                  "da arazi ölçümlerinin epok kararı farklı; önce aynı çerçeveye taşıyın.",
                              f"{at}[{i}]")
        antenna, heights = [], []
        for i, s in enumerate(sols):
            session = self.get("sessions", s["session"])
            a = session.get("antenna")
            if a is None:
                raise Refused("antenna_unknown", f"“{s['session']}” oturumunun anten yüksekliği yok; anteni ve "
                                                 "yüksekliğin ölçüldüğü yeri girin.", f"{at}[{i}]")
            if a.get("east", 0.0) != 0.0 or a.get("north", 0.0) != 0.0:
                raise Refused("antenna_eccentric", f"“{s['session']}” oturumunun anteni işaretin dışında (doğu/kuzey "
                                                   "dış merkezlik); bu sürümde indirgenmez.", f"{at}[{i}]")
            h = antenna_vertical(a)
            if not any(u["session"] == s["session"] for u in antenna):
                antenna.append({"session": s["session"], "height": h})
            heights.append(s["height"] + s["antennaApplied"] - h)
        sigma = None
        if method == "single":
            lat, lon, height, sigma = first["lat"], first["lon"], heights[0], first.get("sigma")
        elif method == "mean":
            a = b = c = 0.0
            for s, h in zip(sols, heights):
                a += s["lat"]
                b += s["lon"]
                c += h
            k = float(n)
            lat, lon, height = a / k, b / k, c / k
        else:
            sums = [[0.0, 0.0] for _ in range(3)]
            for s, h in zip(sols, heights):
                g = s["sigma"]
                for k, v in enumerate((s["lat"], s["lon"], h)):
                    w = 1.0 / (g[k] * g[k])
                    sums[k][0] += w * v
                    sums[k][1] += w
            lat, lon, height = (sums[k][0] / sums[k][1] for k in range(3))
            sigma = [math.sqrt(1.0 / sums[k][1]) for k in range(3)]
        target = self.decision(camp)
        se = first.get("epoch")
        with_z = "heights" in self.d["settings"]
        frm = {"srid": first["srid"], "epoch": se} if se is not None else {"srid": first["srid"]}
        to = {"srid": 5254, "epoch": target} if target is not None else {"srid": 5254}
        if with_z:
            frm["heights"], to["heights"] = ELL, ELL
        point = cv.pt(lon, lat, height) if with_z else cv.pt(lon, lat)
        vel = camp.get("velocity") if camp else None
        if se is not None and target is not None and se != target:
            if vel is None:
                raise Refused("epoch_unsupported", f"Koordinatlar {cv.n(se)} epokundan {cv.n(target)} epokuna "
                                                   "taşınmalı: bunun için hız modeli gerekir. Hız ızgarasını (doğu, "
                                                   "kuzey, yukarı mm/yıl; GeoTIFF) Proje ayarları › Izgaralar'a ekleyip "
                                                   "velocity ile verin; koordinatlar epok farkı yok sayılarak "
                                                   "çevrilmez.", "operation")
            s = gc.transformed([point], frm=frm, to=to, vel=vel)
        else:
            s = cv.transformed([point], frm=frm, to=to)
        out = s["result"]["output"]
        p = out["points"][0]
        r = {"revision": 1, "solutions": list(ids), "method": method, "lat": lat, "lon": lon, "height": height,
             "srid": first["srid"]}
        if se is not None:
            r["epoch"] = se
        if target is not None:
            r["targetEpoch"] = target
        r["antenna"] = antenna
        if sigma is not None:
            r["sigma"] = sigma
        r["x"], r["y"] = p["x"], p["y"]
        if "z" in p:
            r["z"] = p["z"]
        r["chain"] = out["chain"]["status"]
        return r

    # ADR 1740: what a network adjustment's point rests on.
    def network_why(self, a_id):
        a = self.get("adjustments", a_id) if self.survey.get("adjustments") else None
        if a is None:
            return [f"“{a_id}” ağ dengelemesi arşivde yok."]
        why = []
        for bid in a["baselines"]:
            b = self.get("baselines", bid)
            if b is None:
                why.append(f"“{bid}” bazı arşivde yok.")
            elif "excluded" in b:
                why.append(f"“{bid}” bazı dışlandı: {b['excluded']}")
            elif not b["adjust"]:
                why.append(f"“{bid}” bazı dengelemeden çıkarıldı.")
        for kid in a["known"]:
            k = self.get("known", kid)
            if k is None:
                why.append(f"“{kid}” bilinen noktası arşivde yok.")
            elif "excluded" in k:
                why.append(f"“{kid}” bilinen noktası dışlandı: {k['excluded']}")
        return why

    def campaign_of_result(self, r):
        if "adjustment" in r:
            a = self.get("adjustments", r["adjustment"]) if self.survey.get("adjustments") else None
            b = self.get("baselines", a["baselines"][0]) if a else None
            return self.get("campaigns", b["campaign"]) if b else None
        first = self.get("solutions", r["solutions"][0]) if r["solutions"] else None
        return self.campaign_of(first) if first else None

    # §3: why a point is out of date; rows.
    def point_why(self, p):
        why = []
        if "adjustment" in p["result"]:
            why.extend(self.network_why(p["result"]["adjustment"]))
        for used in p["result"]["antenna"]:
            s = self.get("sessions", used["session"])
            now = self.vertical(s) if s else None
            if now != used["height"]:
                if now is None:
                    why.append(f"“{used['session']}” oturumunun anteni yok.")
                else:
                    why.append(f"“{used['session']}” oturumunun anten yüksekliği {m3(used['height'])} m iken "
                               f"{m3(now)} m oldu.")
        now = self.decision(self.campaign_of_result(p["result"]))
        if now != p["result"].get("targetEpoch"):
            why.append(f"Epok kararı {epoch_text(p['result'].get('targetEpoch'))} iken {epoch_text(now)} oldu.")
        for sid in p["result"]["solutions"]:
            s = self.get("solutions", sid)
            if s and "excluded" in s:
                why.append(f"“{sid}” çözümü dışlandı: {s['excluded']}")
        return why

    def point_row(self, p):
        why = self.point_why(p)
        stale = bool(why)
        gone = not self.present(p["uid"])
        if gone:
            why.append("Noktanın nesnesi çizimde yok.")
        row = {"id": p["id"], "mark": p["mark"], "uid": p["uid"],
               "state": "stale" if stale else ("missing" if gone else "current")}
        if why:
            row["why"] = why
        row["result"] = copy.deepcopy(p["result"])
        row["history"] = len(p.get("history", []))
        return row

    def use_row(self, u):
        why = []
        for pr in u["points"]:
            p = self.get("points", pr["point"])
            if p and p["result"]["revision"] != pr["revision"]:
                why.append(f"“{pr['point']}” noktası {pr['revision']}. revizyondan {p['result']['revision']}. "
                           "revizyona geçti.")
        stale = bool(why)
        held = sum(self.present(x) for x in u["uids"])
        gone = len(u["uids"]) - held
        if gone:
            why.append(f"{gone} nesnesi çizimde yok.")
        row = {"id": u["id"], "operation": u["operation"], "objects": held, "recorded": len(u["uids"]),
               "state": "stale" if stale else ("missing" if gone else "current")}
        if why:
            row["why"] = why
        return row

    def stale(self):
        return sum(bool(self.point_why(p)) for p in self.survey["points"]) + \
            sum(self.use_row(u)["state"] == "stale" for u in self.survey["uses"])

    def latest(self):
        best = {}
        for s in self.survey["solutions"]:
            mark = self.mark_of(s)
            session = self.get("sessions", s["session"])
            t = session.get("end") or session.get("start")
            when = survey_time(t) if t else None
            if mark is None or when is None:
                continue
            num = int(s["id"].split("-")[1])
            if mark not in best or when > best[mark][1] or (when == best[mark][1] and num > best[mark][2]):
                best[mark] = (s["id"], when, num)
        return {m: v[0] for m, v in best.items()}

    def solution_row(self, s, latest):
        session = self.get("sessions", s["session"])
        row = {"id": s["id"], "session": s["session"], "kind": s["kind"], "lat": s["lat"], "lon": s["lon"],
               "height": s["height"], "srid": s["srid"]}
        if "epoch" in s:
            row["epoch"] = s["epoch"]
        row["antennaApplied"] = s["antennaApplied"]
        v = self.vertical(session) if session else None
        if v is not None:
            row["markHeight"] = s["height"] + s["antennaApplied"] - v
        for k in ("fix", "epochs", "sigma"):
            if k in s:
                row[k] = s[k]
        row["latest"] = latest == s["id"]
        row["accepted"] = self.point_using(s["id"]) is not None
        row["adjusted"] = bool(s.get("adjust")) and "excluded" not in s
        if "excluded" in s:
            row["excluded"] = s["excluded"]
        return row

    # ADR 4120: a known point's own place in the project, or why it has none.
    def known_place(self, k):
        sv = self.survey
        camp_id = next((b["campaign"] for b in sv.get("baselines", []) if k["mark"] in (b["from"], b["to"])), None)
        camp = self.get("campaigns", camp_id) if camp_id else None
        target = self.decision(camp)
        point = next((p for p in sv["points"] if p["mark"] == k["mark"]), None)
        row = {"id": k["id"], "mark": k["mark"]}
        if point:
            row["point"] = point["id"]
        lat, lon, h = k["lat"], k["lon"], k["height"]
        e = k.get("epoch")
        epoch = e
        if e is not None and target is not None and e != target:
            if k["srid"] not in GEO3D:
                row["why"] = (f"EPSG:{k['srid']} kayıttaki coğrafi bir sistem değil; bilinen noktanın çerçevesini "
                              "(4326 WGS 84, 5252 TUREF) verin.")
                return row
            vel = camp.get("velocity") if camp else None
            if "velocity" in k:
                x0 = geocentric(k["srid"], lat, lon, h)
                now = geographic(k["srid"], [x0[i] + k["velocity"][i] * (target - e) for i in range(3)])
                by = "velocity"
            elif vel is not None:
                s = gc.transformed([cv.pt(lon, lat, h)], frm={"srid": k["srid"], "epoch": e, "heights": ELL},
                                   to={"srid": k["srid"], "epoch": target, "heights": ELL}, vel=vel)
                q = s["result"]["output"]["points"][0]
                now = (q["y"], q["x"], q.get("z", h))
                by = "grid"
            else:
                row["why"] = (f"“{k['id']}” bilinen noktası {epoch_words(e)} epokunda, projenin epok kararı "
                              f"{epoch_words(target)}: projeye taşımak için noktanın kendi hızı (yer merkezli VX, VY, VZ) "
                              "ya da arazi ölçümünün hız ızgarası (Proje ayarları › Izgaralar) gerekir.")
                return row
            a, b = geocentric(k["srid"], lat, lon, h), geocentric(k["srid"], *now)
            r = enu_rows(lat, lon)
            d = [b[i] - a[i] for i in range(3)]
            row["moved"] = {"id": k["id"], "mark": k["mark"], "by": by, "from": e, "years": target - e,
                            "shift": [sum(r[i][j] * d[j] for j in range(3)) for i in range(3)]}
            lat, lon, h = now
            epoch = target
        with_z = "heights" in self.d["settings"]
        frm = {"srid": k["srid"], "epoch": epoch} if epoch is not None else {"srid": k["srid"]}
        to = {"srid": 5254, "epoch": target} if target is not None else {"srid": 5254}
        if with_z:
            frm["heights"], to["heights"] = ELL, ELL
        out = cv.transformed([cv.pt(lon, lat, h) if with_z else cv.pt(lon, lat)], frm=frm, to=to)["result"]["output"]
        p = out["points"][0]
        row["x"], row["y"] = p["x"], p["y"]
        if "z" in p:
            row["z"] = p["z"]
        row["chain"] = out["chain"]["status"]
        if target is not None or epoch is not None:
            row["epoch"] = target if target is not None else epoch
        if point:
            r = point["result"]
            row["dx"], row["dy"] = p["x"] - r["x"], p["y"] - r["y"]
            if "z" in p and "z" in r:
                row["dz"] = p["z"] - r["z"]
            # docs/adr/4520: their horizontal distance.
            row["horizontal"] = math.hypot(row["dx"], row["dy"])
        return row

    def tree(self, deep=False, of=None, files=None, places=False, tolerance=None):
        files = files or {}
        sv = self.survey or {"campaigns": [], "sessions": [], "solutions": [], "points": [], "uses": []}
        self.survey = sv
        work = self.d["settings"].get("mapWork", {}).get("items", [])
        campaigns = []
        for c in sv["campaigns"]:
            row = {"id": c["id"], "name": c["name"]}
            if "work" in c:
                row["work"] = c["work"]
                if not any(i["id"] == c["work"] for i in work):
                    row["workMissing"] = True
            for k in ("operator", "epoch"):
                if k in c:
                    row[k] = c[k]
            if "velocity" in c:
                row["velocity"] = c["velocity"]["name"]
            row["sessions"] = sum(s["campaign"] == c["id"] for s in sv["sessions"])
            campaigns.append(row)
        sessions = []
        for s in sv["sessions"]:
            row = {"id": s["id"], "campaign": s["campaign"], "mark": s["mark"]}
            for k in ("start", "end", "timeSystem", "receiver", "antenna"):
                if k in s:
                    row[k] = copy.deepcopy(s[k])
            v = self.vertical(s)
            if v is not None:
                row["vertical"] = v
            if "file" in s:
                f = s["file"]
                row["file"] = {"path": f["path"], "revision": f["revision"], "format": f["format"],
                               "state": file_state(f, deep, files)}
            row["observations"] = len(s.get("observations", []))
            rp = replay(s)
            if rp:
                row["replay"] = rp
            row["solutions"] = sum(x["session"] == s["id"] for x in sv["solutions"])
            sessions.append(row)
        latest = self.latest()
        marks = []
        for s in sv["sessions"]:
            m = next((m for m in marks if m["mark"] == s["mark"]), None)
            if m:
                m["sessions"].append(s["id"])
            else:
                m = {"mark": s["mark"], "sessions": [s["id"]], "solutions": []}
                p = next((p for p in sv["points"] if p["mark"] == s["mark"]), None)
                if p:
                    m["point"] = p["id"]
                marks.append(m)
        for s in sv["solutions"]:
            mark = self.mark_of(s)
            m = next((m for m in marks if m["mark"] == mark), None)
            if m is not None:
                m["solutions"].append(self.solution_row(s, latest.get(mark)))
        for m in marks:
            if "point" in m:
                m["point"] = m.pop("point")
        points = [self.point_row(p) for p in sv["points"]]
        uses = [self.use_row(u) for u in sv["uses"]]
        out = {"campaigns": campaigns, "sessions": sessions, "marks": marks, "points": points, "uses": uses}
        # ADR 1740: the baselines as they stand, the known points and the network adjustments.
        if sv.get("baselines"):
            rows = []
            for b in sv["baselines"]:
                c = b["covariance"]
                row = {"id": b["id"], "campaign": b["campaign"], "from": b["from"], "to": b["to"],
                       "length": math.sqrt(b["d"][0] * b["d"][0] + b["d"][1] * b["d"][1] + b["d"][2] * b["d"][2]),
                       "sigma": [math.sqrt(c[0]), math.sqrt(c[3]), math.sqrt(c[5])]}
                if "excluded" in b:
                    row["excluded"] = b["excluded"]
                row["adjust"] = b["adjust"]
                if "file" in b:
                    f = b["file"]
                    row["file"] = {"path": f["path"], "revision": f["revision"], "format": f["format"],
                                   "state": file_state(f, deep, files)}
                if "source" in b:
                    row["source"] = b["source"]
                used = [a["id"] for a in sv.get("adjustments", []) if b["id"] in a["baselines"]]
                if used:
                    row["adjustments"] = used
                rows.append(row)
            out["baselines"] = rows
        if sv.get("known"):
            out["known"] = copy.deepcopy(sv["known"])
        if sv.get("adjustments"):
            out["adjustments"] = copy.deepcopy(sv["adjustments"])
        if places and sv.get("known"):
            out["knownPlaces"] = [self.known_place(k) for k in sv["known"]]
            # docs/adr/4520: each place against the user's tolerance: beyond when its horizontal distance passes the
            # horizontal limit or its height difference the vertical one; nothing to judge, nothing said.
            if tolerance:
                for row in out["knownPlaces"]:
                    judged = []
                    if "horizontal" in tolerance and "horizontal" in row:
                        judged.append(row["horizontal"] > tolerance["horizontal"])
                    if "vertical" in tolerance and "dz" in row:
                        judged.append(abs(row["dz"]) > tolerance["vertical"])
                    if judged:
                        row["beyond"] = any(judged)
        if of is not None:
            out["object"] = self.object(of, latest)
        out["stale"] = sum(p["state"] == "stale" for p in points) + sum(u["state"] == "stale" for u in uses)
        out["deep"] = deep
        out["revision"] = "$current"
        return out

    def object(self, u, latest):
        p = next((p for p in self.survey["points"] if p["uid"] == u), None)
        o = {"uid": u}
        if p:
            o["point"] = self.point_row(p)
            o["solutions"] = [self.solution_row(self.get("solutions", s), latest.get(p["mark"]))
                              for s in p["result"]["solutions"]]
        used = next((x for x in self.survey["uses"] if u in x["uids"]), None)
        if used:
            o["used"] = self.use_row(used)
        return o


def replay(s):
    obs = s.get("observations", [])
    if not obs:
        return None
    fixes = []
    for o in obs:
        f = o.get("fix", "—")
        e = next((x for x in fixes if x["fix"] == f), None)
        if e:
            e["count"] += 1
        else:
            fixes.append({"fix": f, "count": 1})
    drops = sum(1 for a, b in zip(obs, obs[1:]) if a.get("fix") == "RTK sabit" and b.get("fix") != "RTK sabit")
    timed = [seconds(o["time"]) for o in obs if "time" in o and seconds(o["time"]) is not None]
    steps = [math.floor((b - a) * 1000.0 + 0.5) for a, b in zip(timed, timed[1:])]
    tally = []
    for st in steps:
        if st > 0:
            e = next((t for t in tally if t[0] == st), None)
            if e:
                e[1] += 1
            else:
                tally.append([st, 1])
    nominal = None
    if tally:
        nominal = sorted(tally, key=lambda t: (-t[1], t[0]))[0][0]
    jumps = sum(1 for st in steps if st <= 0 or (nominal is not None and st > 2 * nominal))
    out = {"fixes": fixes, "floatDrops": drops, "timeJumps": jumps}
    if nominal is not None:
        out["nominal"] = nominal / 1000.0
    return out


def file_state(f, deep, files):
    path = f["path"]
    now = files.get(path)
    if now is None and path.startswith("$fixture:map-survey/v1/"):
        # The network's files (map_survey_network_cases.py, ADR 1740) are written there by their own script.
        name = path.split("/")[-1]
        now = sha(WRITTEN[name]) if name in WRITTEN else sha((FILES / name).read_bytes())
    if now is None:
        return "missing"
    if not deep:
        return "found"
    return "current" if now == f["revision"] else "changed"


def written(survey):
    return {k: v for k, v in survey.items() if v}


def with_any_xy(survey):
    """The archive as a case expects it in the settings: the projected x and y PROJ gave are compared in the
    results and objects within 1e-6 m, here anything."""
    s = copy.deepcopy(written(survey))
    for p in s.get("points", []):
        for r in [p["result"], *p.get("history", [])]:
            r["x"], r["y"] = "$any", "$any"
            if "z" in r:
                r["z"] = "$any"
    return s


# ── Steps and answers ────────────────────────────────────────────────────────────────────────────────────────────

UNCHANGED = {"revision": "same", "canUndo": False}
NOT_MAP = {"status": "failed", "error": {
    "code": "not_a_map_project",
    "message": "GNSS arşivi yalnız harita projesinde (KentOS Harita) olur; bu çizim harita projesi değil."}}
BAD_REVISION = {"status": "failed", "error": {
    "code": "invalid_revision",
    "message": "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden "
               "ya da komutun planından alın.", "path": "expectedRevision"}}
CONFLICT = {"status": "conflict", "error": {
    "code": "revision_conflict",
    "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre "
               "yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}}
WITHIN = 1e-6


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def refused(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_, result, expect=None, command=None, within=None):
    s = {"op": op, "input": input_, "result": result}
    if command:
        s["command"] = command
    if expect is not None:
        s["expect"] = expect
    if within is not None:
        s["within"] = within
    return s


def case(name, steps, note=None, d=None):
    c = {"name": name}
    if note:
        c["note"] = note
    c["steps"] = steps
    if d is not None:
        c["setup"] = d.document()
    return c


def ask(m, deep=False, of=None, op="execute", expect=None, files=None, places=False, tolerance=None):
    input_ = {}
    if deep:
        input_["deep"] = True
    if of is not None:
        input_["uid"] = of
    if places:
        input_["places"] = True
    if tolerance is not None:
        input_["tolerance"] = tolerance
    return step(op, input_, done(m.tree(deep, of, files, places, tolerance)), expect or UNCHANGED,
                command="map.survey.query", within=WITHIN)


def query_cases(base):
    out = []
    out.append(case("Bütün arşiv: arazi ölçümü, iki günlük N1 ve NMEA'lı N2, kabul edilen N1 ve ona dayanan yüzey",
                    [ask(base)],
                    "N1'in son ölçüleni ikinci günün çözümü; cz-4 dışlanmış (kayıtta); N2'nin oynatma özeti: RTK "
                    "sabitten bir düşüş, bir zaman atlaması, olağan adım 1 sn."))
    out.append(case("Derin okuma oturumların dosyalarının SHA-256'sını karşılaştırır", [ask(base, deep=True)]))
    out.append(case("Kabul edilen noktanın ölçüsü: sonucu ve çözümleri", [ask(base, of=P1)]))
    out.append(case("Yüzeyin çizgisi kabul edilen noktaya dayanır", [ask(base, of=S1)]))
    out.append(case("Arşivin adlandırmadığı çizgi: yalnız kimliği", [ask(base, of=F1)]))
    out.append(case("Kimlik UUID değil", [step("execute", {"uid": "P1"}, refused(
        "invalid_uid", "“P1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "uid"),
        UNCHANGED, command="map.survey.query")]))
    out.append(case("Çizimde olmayan nesne", [step("execute", {"uid": GHOST}, refused(
        "entity_not_found", f"“{GHOST}” kimlikli nesne çizimde yok.", "uid"), UNCHANGED, command="map.survey.query")]))
    arch = setup(work="architecture", survey=None)
    out.append(case("Harita projesi değil", [step("execute", {}, NOT_MAP, UNCHANGED, command="map.survey.query")],
                    d=arch))
    empty = setup(survey=None)
    out.append(case("Arşivi olmayan harita projesi: boş ağaç", [ask(empty)], d=empty))
    gone = base.clone()
    gone.survey["sessions"][2]["file"]["path"] = "$tmp:tasindi-1340/n2.nmea"
    out.append(case("N2'nin kayıt dosyası yerinde değil: missing; ham ölçümler projede kalır", [ask(gone)], d=gone))
    changed = base.clone()
    changed.survey["sessions"][0]["file"]["path"] = "$tmp:degisti-1340/n1-gun1.26o"
    files = {"$tmp:degisti-1340/n1-gun1.26o": sha(GUN2)}
    out.append(case("İlk günün RINEX dosyası sonradan değişti: hızlı okuma found, derin okuma changed", [
        {"op": "copyFile", "from": fx("n1-gun2.26o"), "to": "$tmp:degisti-1340/n1-gun1.26o"},
        ask(changed, files=files), ask(changed, deep=True, files=files)], d=changed))
    ant = base.clone()
    ant.survey["sessions"][1]["antenna"]["height"] = 1.81
    out.append(case("İkinci günün anten yüksekliği düzeltildi: kabul edilen N1 güncel değil, neden yazılı", [ask(ant)],
                    d=ant))
    ep = base.clone()
    ep.survey["campaigns"][0]["epoch"] = 2026.85
    out.append(case("Arazi ölçümünün epok kararı değişti: N1 güncel değil", [ask(ep)], d=ep))
    ex = base.clone()
    ex.survey["solutions"][1]["excluded"] = "Anten yüksekliği defterde şüpheli"
    out.append(case("Kabulden sonra çözümü dışlandı: N1 güncel değil; çözüm kayıtta kalır", [ask(ex, of=P1)], d=ex))
    moved = base.clone()
    p = moved.survey["points"][0]
    p["history"] = [copy.deepcopy(p["result"])]
    p["result"]["revision"] = 2
    p["result"]["reasons"] = ["Anten yüksekliği düzeltildi."]
    out.append(case("Nokta yeni revizyona geçti: ona dayanan yüzey güncel değil", [ask(moved, of=S1)], d=moved))
    deleted = base.clone()
    del deleted.uids[1]
    out.append(case("Kabul edilen noktanın nesnesi silindi: missing; geri alınca güncel", [
        step("execute", {"uids": [P1]}, "$any", command="cad.entities.delete"),
        ask(deleted, expect={"revision": "same"}), {"op": "undo", "returns": "Sil"},
        ask(base, expect={"revision": "same"})]))
    wm = base.clone()
    wm.survey["campaigns"][0]["work"] = "is-9"
    out.append(case("Arazi ölçümünün bağlı olduğu harita işi öğesi yok: workMissing", [ask(wm)], d=wm))
    tie = base.clone()
    tie.survey["sessions"][0]["end"] = tie.survey["sessions"][1]["end"]
    out.append(case("İki oturumun bitişi aynı: son ölçülen kimlik numarası büyük olan", [ask(tie)], d=tie))
    out.append(case("Plan yazmaz, aynı ağacı söyler", [ask(base, op="plan")]))
    out.append(case("Doğrulama yazmaz", [step("validate", {}, done(None), UNCHANGED, command="map.survey.query")]))
    out.append(case("Doğrulama kimliği de denetler", [step("validate", {"uid": GHOST}, refused(
        "entity_not_found", f"“{GHOST}” kimlikli nesne çizimde yok.", "uid"), UNCHANGED, command="map.survey.query")]))
    nohz = setup(heights=False)
    nohz.survey = None
    nohz2 = setup(heights=False, survey=None)
    nohz2.survey = start_survey()
    r = nohz2.compute("N1", ["cz-1", "cz-2"], "mean")
    nohz2.survey["points"].append({"id": "kn-1", "mark": "N1", "uid": P1, "result": r})
    nohz2.entities[1] = point_entity("gnss", r, "N1", "kn-1")
    nohz2.uids[1] = P1
    out.append(case("Projenin yükseklikleri söylenmemiş: kabul noktası kotsuz, işaret yüksekliği arşivde",
                    [ask(nohz2, of=P1)], d=nohz2))
    return out


# ── map.survey.edit ──────────────────────────────────────────────────────────────────────────────────────────────


def op(o, **kw):
    return {"operation": dict(o, **kw)}


def edited(m, **kw):
    out = {}
    for k in ("id", "uid", "pointRevision", "observations", "removed"):
        if kw.get(k):
            out[k] = kw[k]
    out["stale"] = m.stale()
    out["revision"] = "$current"
    return out


def edit(input_, result, expect, op_="execute", within=None):
    return step(op_, input_, result, expect, command="map.survey.edit", within=within)


def fail_step(input_, code, message, path, op_="execute", after_write=False):
    return edit(input_, refused(code, message, path), {"revision": "same"} if after_write else UNCHANGED, op_)


def write_step(before, after, input_, fields, label, warnings=(), extra=None, then_undo=True):
    expect = {"revision": "changed", "canUndo": True, "settings": {"mapSurvey": with_any_xy(after.survey)}}
    expect.update(extra or {})
    steps = [edit(input_, done(edited(after, **fields), warnings), expect, within=WITHIN)]
    if then_undo:
        steps.append({"op": "undo", "returns": label,
                      "expect": {"canRedo": True, "settings": {"mapSurvey": with_any_xy(before.survey)},
                                 "ids": sorted(before.entities)}})
    return steps


def warn(code, message, path):
    return {"code": code, "message": message, "path": path}


def changed(base, fn):
    m = base.clone()
    fn(m)
    return m


def edit_cases(base):
    out = []
    S = lambda m: m.survey  # noqa: E731
    # campaign
    vel = {"name": HIZ["name"], "sha256": HIZ["sha256"]}
    after = changed(base, lambda m: S(m)["campaigns"].append({"id": "kp-2", "name": "GNSS ölçümü 2027-03",
                                                              "operator": "Ayşe Demir", "epoch": 2027.2,
                                                              "velocity": vel}))
    out.append(case("Arazi ölçümü ekle: kp-2, operatör, epok kararı ve hız ızgarasıyla; tek adımda geri alınır",
                    write_step(base, after, op({"op": "campaign", "name": "GNSS ölçümü 2027-03",
                                                "operator": "Ayşe Demir", "epoch": 2027.2, "velocity": vel}),
                               {"id": "kp-2"}, "Arazi ölçümü ekle")))
    out.append(case("Arazi ölçümü: boş ad", [fail_step(op({"op": "campaign", "name": ""}), "invalid_survey_text",
                                                   "Arazi ölçümünün adı boşlukla başlamayan ve bitmeyen 1–200 karakter "
                                                   "olmalı.", "operation.name")]))
    out.append(case("Arazi ölçümü: harita işinde olmayan öğe", [fail_step(
        op({"op": "campaign", "name": "Yeni", "work": "is-9"}), "unknown_work_item",
        "Harita işinde “is-9” kimlikli arazi ölçümü öğesi yok.", "operation.work")]))
    out.append(case("Arazi ölçümü: epok aralık dışında", [fail_step(
        op({"op": "campaign", "name": "Yeni", "epoch": 1850.0}), "invalid_survey_value",
        "Epok: koordinat epoku 1900 ile 2100 arasında ondalık bir yıl olmalı (2023.5 gibi).", "operation.epoch")]))
    after = changed(base, lambda m: S(m)["campaigns"][0].update(epoch=2026.85))
    out.append(case("Epok kararı 2026,85: N1 güncel değil (çıktıda bir eski)",
                    write_step(base, after, op({"op": "setCampaign", "campaign": "kp-1", "epoch": 2026.85}),
                               {"id": "kp-1"}, "Arazi ölçümünü değiştir")))
    out.append(case("Arazi ölçümünü değiştir: aynı değer", [fail_step(
        op({"op": "setCampaign", "campaign": "kp-1", "epoch": 2005.0}), "nothing_to_set", "Değişecek bir şey yok.",
        "operation")]))
    out.append(case("Arazi ölçümünü değiştir: olmayan arazi ölçümü", [fail_step(
        op({"op": "setCampaign", "campaign": "kp-9", "name": "X"}), "unknown_survey_record",
        "GNSS arşivinde “kp-9” kimlikli arazi ölçümü yok.", "operation.campaign")]))
    # session
    h3, _ = read_rinex(GUN3)
    s4 = {"id": "os-4", "campaign": "kp-1", "mark": "N3", "start": h3["start"], "timeSystem": "gps",
          "receiver": h3["receiver"], "antenna": h3["antenna"],
          "file": {"path": fx("n3-gun3.26o"), "revision": sha(GUN3), "format": "rinex"}, "rinex": h3["facts"]}
    after = changed(base, lambda m: S(m)["sessions"].append(copy.deepcopy(s4)))
    out.append(case("RINEX 4.00'dan oturum: işaret, alıcı, anten (doğu dış merkezliği kayıtta), başlangıç ve zaman "
                    "sistemi başlıktan; aynı işaretin üstüne yazılmaz",
                    write_step(base, after, op({"op": "session", "campaign": "kp-1", "file": fx("n3-gun3.26o")}),
                               {"id": "os-4"}, "GNSS oturumu ekle")))
    s1 = session_from_rinex("os-4", "n1-gun1.26o")
    given = {"type": "TRM57971.00     NONE", "height": 1.55}
    s1["antenna"] = given
    after = changed(base, lambda m: S(m)["sessions"].append(copy.deepcopy(s1)))
    out.append(case("RINEX'ten oturum, defterdeki anten yüksekliğiyle: verilen yazılır, fark uyarıdır",
                    write_step(base, after, op({"op": "session", "campaign": "kp-1", "file": fx("n1-gun1.26o"),
                                                "antenna": given}),
                               {"id": "os-4"}, "GNSS oturumu ekle",
                               warnings=[warn("rinex_differs", "RINEX başlığı başka değer söylüyor; verilen yazıldı: "
                                                               "anten yüksekliği (1.532 m).", "operation.file")])))
    s5 = session_from_nmea("os-4", "n2.nmea", "N2b")
    read = nmea_points("n2.nmea")
    after = changed(base, lambda m: S(m)["sessions"].append(copy.deepcopy(s5)))
    out.append(case("NMEA kaydından oturum: kesik cümle okunmaz, okunan altı konum ve ilk/son zaman (UTC) yazılır",
                    write_step(base, after, op({"op": "session", "campaign": "kp-1", "mark": "N2b",
                                                "file": fx("n2.nmea")}),
                               {"id": "os-4", "observations": len(read["points"])}, "GNSS oturumu ekle",
                               warnings=[warn("gnss_lines_skipped",
                                              f"“n2.nmea” dosyasının {len(read['problems'])} satırı okunmadı; okunan "
                                              f"{len(read['points'])} konum yazıldı. İlki: "
                                              f"{read['problems'][0]['problem']}", "operation.file")])))
    out.append(case("Oturum: NMEA'nın işareti yok, işaret verilmedi", [fail_step(
        op({"op": "session", "campaign": "kp-1", "file": fx("n2.nmea")}), "invalid_survey_text",
        "Oturumun işareti verilmedi ve dosya söylemiyor; işaretin adını verin.", "operation.mark")]))
    for name, why in (("nav.26n", "RINEX dosyası gözlem dosyası değil (tür “N”); oturum gözlem dosyasından okunur."),
                      ("kesik.26o", "RINEX başlığının sonu (END OF HEADER) yok; dosya kesilmiş olabilir.")):
        path = fx(name)
        out.append(case(f"Oturum: {name} okunmaz", [fail_step(
            op({"op": "session", "campaign": "kp-1", "file": path}), "unreadable_gnss_file", f"“{path}”: {why}",
            "operation.file")]))
    out.append(case("Oturum: konumsuz NMEA", [fail_step(
        op({"op": "session", "campaign": "kp-1", "mark": "N9", "file": fx("bos.nmea")}), "unreadable_gnss_file",
        f"“{fx('bos.nmea')}” dosyasında okunan konum yok: RINEX gözlem dosyası, GPX ya da NMEA kaydı değil",
        "operation.file")]))
    out.append(case("Oturum: göreli yol", [fail_step(
        op({"op": "session", "campaign": "kp-1", "mark": "N9", "file": "veri/n9.26o"}), "invalid_file_path",
        "“veri/n9.26o” mutlak bir dosya yolu değil; dosyanın tam yolunu verin.", "operation.file")]))
    out.append(case("Oturum: olmayan dosya", [fail_step(
        op({"op": "session", "campaign": "kp-1", "mark": "N9", "file": "$tmp:yok-1340/n9.26o"}), "file_not_found",
        "“$tmp:yok-1340/n9.26o” dosyası yok.", "operation.file")]))
    out.append(case("Oturum: eğik yükseklik yarıçapsız", [fail_step(
        op({"op": "session", "campaign": "kp-1", "mark": "N9", "antenna": {"height": 1.6, "reference": "slant"}}),
        "invalid_survey_value", "Anten: eğik yükseklikte antenin yarıçapı 0'dan büyük, yükseklikten küçük olmalı.",
        "operation.antenna.radius")]))
    out.append(case("Oturum: zaman biçimi", [fail_step(
        op({"op": "session", "campaign": "kp-1", "mark": "N9", "start": "2026-11-03 09:30"}), "invalid_survey_value",
        "“2026-11-03 09:30” geçerli bir zaman değil; YYYY-AA-GGTss:dd:sn[.kesir]Z biçiminde yazın.",
        "operation.start")]))
    out.append(case("Oturum: bitiş başlangıçtan önce", [fail_step(
        op({"op": "session", "campaign": "kp-1", "mark": "N9", "start": "2026-11-03T10:00:00Z",
            "end": "2026-11-03T09:00:00Z"}), "invalid_survey_value", "Bitiş başlangıçtan önce olamaz.",
        "operation.end")]))
    out.append(case("Oturum: olmayan arazi ölçümü", [fail_step(
        op({"op": "session", "campaign": "kp-7", "mark": "N9"}), "unknown_survey_record",
        "GNSS arşivinde “kp-7” kimlikli arazi ölçümü yok.", "operation.campaign")]))
    # setSession
    slant = {"type": "LEIGS18         NONE", "height": 1.84, "reference": "slant", "radius": 0.12, "offset": 0.05}
    after = changed(base, lambda m: S(m)["sessions"][1].update(antenna=copy.deepcopy(slant)))
    out.append(case("İkinci günün anteni eğik ölçülmüş: √(S²−R²)−offset; N1 güncel değil",
                    write_step(base, after, op({"op": "setSession", "session": "os-2", "antenna": slant}),
                               {"id": "os-2"}, "Oturumu değiştir")))
    out.append(case("Oturumun işareti değişmez: çözümü kabul edilen noktada", [fail_step(
        op({"op": "setSession", "session": "os-1", "mark": "N1A"}), "survey_in_use",
        "“os-1” oturumunun çözümü “kn-1” noktasında; oturumun işareti değişmez.", "operation.mark")]))
    out.append(case("Oturumu değiştir: aynı değer", [fail_step(
        op({"op": "setSession", "session": "os-3", "mark": "N2"}), "nothing_to_set", "Değişecek bir şey yok.",
        "operation")]))
    # solve
    s3 = base.survey["sessions"][2]
    cz5 = receiver_solution("cz-5", s3, 5252, 0.0, 1, 3, [], epoch=2005.0)
    after = changed(base, lambda m: S(m)["solutions"].append(cz5))
    out.append(case("Alıcı çözümü: 1–3 konumları, bütün fix'ler (fix karışık: yazılmaz), TUREF 2005,0",
                    write_step(base, after, op({"op": "solve", "session": "os-3", "srid": 5252, "antennaApplied": 0.0,
                                                "epoch": 2005.0, "from": 1, "to": 3}),
                               {"id": "cz-5"}, "Alıcı çözümü")))
    out.append(case("Alıcı çözümü: RINEX oturumunda ham ölçüm yok", [fail_step(
        op({"op": "solve", "session": "os-1", "srid": 5252, "antennaApplied": 0.0}), "no_observations",
        "“os-1” oturumunda ham ölçüm yok; NMEA ya da GPX dosyasıyla eklenen oturum çözülür.", "operation.session")]))
    out.append(case("Alıcı çözümü: istenen fix yok", [fail_step(
        op({"op": "solve", "session": "os-3", "srid": 4326, "antennaApplied": 0.0, "fixes": ["DGPS"]}),
        "no_observations", "Seçilen aralıkta ve fix'lerde ham ölçüm yok.", "operation.fixes")]))
    out.append(case("Alıcı çözümü: çerçeve projeksiyonlu sistem", [fail_step(
        op({"op": "solve", "session": "os-3", "srid": 5254, "antennaApplied": 0.0}), "unknown_frame",
        "EPSG:5254 kayıttaki coğrafi bir sistem değil; çözümün çerçevesini (4326 WGS 84, 5252 TUREF) verin.",
        "operation.srid")]))
    out.append(case("Alıcı çözümü: aralık konumların dışında", [fail_step(
        op({"op": "solve", "session": "os-3", "srid": 4326, "antennaApplied": 0.0, "to": 6}), "invalid_survey_value",
        "Aralık oturumun 0–5 konumları içinde olmalı.", "operation.to")]))
    out.append(case("Alıcı çözümü: seçilenin hiçbirinin elipsoit yüksekliği yok", [fail_step(
        op({"op": "solve", "session": "os-3", "srid": 4326, "antennaApplied": 0.0, "from": 5, "to": 5}),
        "no_ellipsoidal_height", "Seçilen ham ölçümlerin hiçbirinin elipsoit yüksekliği yok (GPX'in yüksekliğinin "
                                 "datumu bilinmez); başka aralık ya da fix seçin.", "operation.from")]))
    cz6 = receiver_solution("cz-5", s3, 4326, 0.0, 0, 5, ["RTK sabit"])
    after = changed(base, lambda m: S(m)["solutions"].append(cz6))
    out.append(case("Alıcı çözümü: bütün RTK sabit ölçümler; yüksekliği olmayan son ölçüm dışarıda kalır (4 epok)",
                    write_step(base, after, op({"op": "solve", "session": "os-3", "srid": 4326, "antennaApplied": 0.0,
                                                "fixes": ["RTK sabit"]}), {"id": "cz-5"}, "Alıcı çözümü")))
    # solution
    sol = {"session": "os-2", "kind": "ppp", "lat": 39.7767127, "lon": 30.5206452, "height": 846.861, "srid": 5252,
           "antennaApplied": 0.0, "epoch": 2005.0, "sigma": [0.012, 0.011, 0.03],
           "covariance": [0.000144, 1e-05, 0.0, 0.000121, 0.0, 0.0009], "correction": "AUSPOS",
           "report": fx("rapor-n1.txt"), "recipe": "AUSPOS 3.0; IGS20 son yörünge; 24 saat"}
    cz = {"id": "cz-5", **{k: v for k, v in sol.items() if k != "report"},
          "report": {"path": fx("rapor-n1.txt"), "revision": sha(REPORT), "format": "report"}}
    after = changed(base, lambda m: S(m)["solutions"].append(cz))
    out.append(case("PPP çözümü: tam kovaryans, rapor dosyası, yeniden işleme tarifiyle",
                    write_step(base, after, op({"op": "solution", **sol}), {"id": "cz-5"}, "Çözüm ekle")))
    bad = dict(sol, covariance=[0.0001, 0.0002, 0.0, 0.0001, 0.0, 0.0004])
    out.append(case("Çözüm: kovaryans pozitif tanımlı değil", [fail_step(
        op({"op": "solution", **bad}), "invalid_survey_value",
        "Çözüm: kovaryans pozitif tanımlı olmalı (köşegen, 2×2 ve 3×3 öncü küçükler 0'dan büyük).",
        "operation.covariance")]))
    out.append(case("Çözüm: enlem 95°", [fail_step(
        op({"op": "solution", **dict(sol, lat=95.0)}), "invalid_survey_value",
        "Çözüm: enlem −90 ile 90 derece arasında olmalı.", "operation.lat")]))
    # exclude, include, adjust
    reason = "Ağaç altında; float'a düştü"
    after = changed(base, lambda m: S(m)["solutions"][1].update(excluded=reason))
    out.append(case("Dışla: ikinci günün çözümü nedeniyle; silinmez, N1 güncel değil",
                    write_step(base, after, op({"op": "exclude", "solution": "cz-2", "reason": reason}),
                               {"id": "cz-2"}, "Çözümü dışla")))
    out.append(case("Dışla: boş neden", [fail_step(
        op({"op": "exclude", "solution": "cz-2", "reason": " "}), "invalid_survey_text",
        "Dışlama nedeni boşlukla başlamayan ve bitmeyen 1–500 karakter olmalı.", "operation.reason")]))
    after = changed(base, lambda m: S(m)["solutions"][3].pop("excluded"))
    out.append(case("Dışlamayı kaldır: PPP çözümü yeniden kullanılabilir",
                    write_step(base, after, op({"op": "include", "solution": "cz-4"}), {"id": "cz-4"},
                               "Dışlamayı kaldır")))
    out.append(case("Dışlamayı kaldır: dışlanmamış çözüm", [fail_step(
        op({"op": "include", "solution": "cz-1"}), "nothing_to_set", "Değişecek bir şey yok.", "operation")]))
    out.append(case("Dengeye kat: dışlanmış çözüm girmez", [fail_step(
        op({"op": "adjust", "solution": "cz-4", "adjust": True}), "solution_excluded",
        "“cz-4” çözümü dışlanmış; dengelemeye girmez. Önce dışlamayı kaldırın.", "operation.adjust")]))
    after = changed(base, lambda m: S(m)["solutions"][0].pop("adjust"))
    out.append(case("Dengeden çıkar: ilk günün çözümü", write_step(
        base, after, op({"op": "adjust", "solution": "cz-1", "adjust": False}), {"id": "cz-1"}, "Dengeleme seçimi")))
    # accept
    r = base.compute("N2", ["cz-3"], "single")
    after = changed(base, lambda m: None)
    after.survey["points"].append({"id": "kn-2", "mark": "N2", "uid": "$uidOf:5", "result": r})
    after.entities[5] = point_entity("gnss", r, "N2", "kn-2")
    after.uids[5] = "new"
    e = point_entity("gnss", r, "N2", "kn-2")
    out.append(case("Kabul et: N2'nin RTK çözümü tek çözüm; nokta nesnesi ve kn-2 tek adımda; WGS 84 zinciri sınırlı",
                    write_step(base, after, op({"op": "accept", "mark": "N2", "solutions": ["cz-3"]}),
                               {"id": "kn-2", "uid": "$uidOf:5", "pointRevision": 1}, "Noktayı kabul et",
                               extra={"ids": [1, 2, 3, 4, 5],
                                      "fields": {"5": {"layerId": "gnss", "p": e["p"], "z": e["z"],
                                                       "attrs": e["attrs"]}}})))
    out.append(case("Kabul et: işaretin noktası var", [fail_step(
        op({"op": "accept", "mark": "N1", "solutions": ["cz-1"]}), "already_accepted",
        "“N1” işaretinin kabul edilen noktası var (“kn-1”); Yeniden üret ile güncelleyin.", "operation.mark")]))
    out.append(case("Kabul et: başka işaretin çözümü", [fail_step(
        op({"op": "accept", "mark": "N2", "solutions": ["cz-1"]}), "mark_mismatch",
        "“cz-1” çözümü “N1” işaretinin, “N2” işaretinin değil.", "operation.solutions[0]")]))
    out.append(case("Kabul et: ortalama tek çözümle", [fail_step(
        op({"op": "accept", "mark": "N2", "solutions": ["cz-3"], "method": "mean"}), "method_count",
        "Ortalama yöntemi en az iki çözüm ister; 1 çözüm verildi.", "operation.solutions")]))
    out.append(case("Kabul et: dışlanmış çözüm", [fail_step(
        op({"op": "accept", "mark": "N2", "solutions": ["cz-4"]}), "solution_excluded",
        "“cz-4” çözümü dışlanmış (Çok yollu yansıma şüphesi; 40 dakikalık veri); önce dışlamayı kaldırın ya da başka "
        "çözüm seçin.", "operation.solutions[0]")]))
    mid = changed(base, lambda m: S(m)["solutions"].append(cz5))
    out.append(case("Kabul et: ağırlıklı ortalama σ'sız çözümle", [
        *write_step(base, mid, op({"op": "solve", "session": "os-3", "srid": 5252, "antennaApplied": 0.0,
                                   "epoch": 2005.0, "from": 1, "to": 3}), {"id": "cz-5"}, "Alıcı çözümü",
                    then_undo=False),
        fail_step(op({"op": "accept", "mark": "N2", "solutions": ["cz-3", "cz-5"], "method": "weighted"}),
                  "sigma_missing", "Ağırlıklı ortalama her çözümün σ'sını ister; “cz-3” çözümünün σ'sı yok.",
                  "operation.solutions[0]", after_write=True)]))
    out.append(case("Kabul et: WGS 84 ve TUREF çözümü birlikte", [
        *write_step(base, mid, op({"op": "solve", "session": "os-3", "srid": 5252, "antennaApplied": 0.0,
                                   "epoch": 2005.0, "from": 1, "to": 3}), {"id": "cz-5"}, "Alıcı çözümü",
                    then_undo=False),
        fail_step(op({"op": "accept", "mark": "N2", "solutions": ["cz-3", "cz-5"]}), "solutions_differ",
                  "“cz-3” ile “cz-5” aynı çerçeve ve epokta değil ya da arazi ölçümlerinin epok kararı farklı; önce aynı "
                  "çerçeveye taşıyın.", "operation.solutions[1]", after_write=True)]))
    n3 = changed(base, lambda m: S(m)["sessions"].append(copy.deepcopy(s4)))
    n3sol = {"session": "os-4", "kind": "processed", "lat": 39.77, "lon": 30.51, "height": 840.0, "srid": 5252,
             "epoch": 2005.0, "antennaApplied": 0.0}
    n3b = changed(n3, lambda m: S(m)["solutions"].append({"id": "cz-5", **n3sol}))
    out.append(case("Kabul et: dış merkezli anten indirgenmez", [
        *write_step(base, n3, op({"op": "session", "campaign": "kp-1", "file": fx("n3-gun3.26o")}), {"id": "os-4"},
                    "GNSS oturumu ekle", then_undo=False),
        *write_step(n3, n3b, op({"op": "solution", **n3sol}), {"id": "cz-5"}, "Çözüm ekle", then_undo=False),
        fail_step(op({"op": "accept", "mark": "N3", "solutions": ["cz-5"]}), "antenna_eccentric",
                  "“os-4” oturumunun anteni işaretin dışında (doğu/kuzey dış merkezlik); bu sürümde indirgenmez.",
                  "operation.solutions[0]", after_write=True)]))
    n9 = changed(base, lambda m: S(m)["sessions"].append({"id": "os-4", "campaign": "kp-1", "mark": "N9"}))
    n9sol = dict(n3sol, session="os-4")
    n9b = changed(n9, lambda m: S(m)["solutions"].append({"id": "cz-5", **n9sol}))
    out.append(case("Kabul et: oturumun anteni yok", [
        *write_step(base, n9, op({"op": "session", "campaign": "kp-1", "mark": "N9"}), {"id": "os-4"},
                    "GNSS oturumu ekle", then_undo=False),
        *write_step(n9, n9b, op({"op": "solution", **n9sol}), {"id": "cz-5"}, "Çözüm ekle", then_undo=False),
        fail_step(op({"op": "accept", "mark": "N9", "solutions": ["cz-5"]}), "antenna_unknown",
                  "“os-4” oturumunun anten yüksekliği yok; anteni ve yüksekliğin ölçüldüğü yeri girin.",
                  "operation.solutions[0]", after_write=True)]))
    out.append(case("Kabul et: kilitli katman", [fail_step(
        op({"op": "accept", "mark": "N2", "solutions": ["cz-3"], "layer": "kilitli"}), "layer_locked",
        "“Kilitli” katmanı kilitli: kabul edilen nokta kilitli katmana çizilmez. Kilidi açın.", "operation.layer")]))
    out.append(case("Kabul et: olmayan katman", [fail_step(
        op({"op": "accept", "mark": "N2", "solutions": ["cz-3"], "layer": "yok"}), "layer_not_found",
        "“yok” kimlikli katman çizimde yok.", "operation.layer")]))
    # regenerate
    fixed = changed(base, lambda m: S(m)["sessions"][1]["antenna"].update(height=1.81))
    regen = fixed.clone()
    p = regen.survey["points"][0]
    r2 = regen.compute("N1", ["cz-1", "cz-2"], "mean")
    r2["revision"] = 2
    r2["reasons"] = fixed.point_why(fixed.survey["points"][0])
    p["history"] = [copy.deepcopy(p["result"])]
    p["result"] = r2
    e2 = point_entity("gnss", r2, "N1", "kn-1")
    regen.entities[1] = e2
    steps = write_step(base, fixed, op({"op": "setSession", "session": "os-2",
                                        "antenna": dict(base.survey["sessions"][1]["antenna"], height=1.81)}),
                       {"id": "os-2"}, "Oturumu değiştir", then_undo=False)
    steps += write_step(fixed, regen, op({"op": "regenerate", "point": "kn-1"}),
                        {"id": "kn-1", "uid": P1, "pointRevision": 2}, "Noktayı yeniden üret",
                        extra={"fields": {"1": {"p": e2["p"], "z": e2["z"], "attrs": e2["attrs"]}}})
    steps.append({"op": "redo", "returns": "Noktayı yeniden üret"})
    steps.append(ask(regen, of=S1, expect={"revision": "same"}))
    out.append(case("Anten kararı değişti, yeniden üret: eski sonuç geçmişte, nesne yeni kotta, yüzey güncel değil; "
                    "geri al ve yinele", steps,
                    "Kabulün ölçütü: iki oturum ve iki çözüm bir noktada; anten yüksekliği değişince nokta güncel "
                    "değil; yeniden üretilen noktanın eski raporu korunur, ona dayanan yüzey güncel değil."))
    wtd = base.clone()
    pw = wtd.survey["points"][0]
    rw = wtd.compute("N1", ["cz-1", "cz-2"], "weighted")
    rw["revision"] = 2
    rw["reasons"] = ["Yöntem Ortalama iken Ağırlıklı ortalama oldu."]
    pw["history"] = [copy.deepcopy(pw["result"])]
    pw["result"] = rw
    ew = point_entity("gnss", rw, "N1", "kn-1")
    wtd.entities[1] = ew
    out.append(case("Yeniden üret: ağırlıklı ortalama (1/σ²), σ sonuçta",
                    write_step(base, wtd, op({"op": "regenerate", "point": "kn-1", "method": "weighted"}),
                               {"id": "kn-1", "uid": P1, "pointRevision": 2}, "Noktayı yeniden üret",
                               extra={"fields": {"1": {"p": ew["p"], "z": ew["z"], "attrs": ew["attrs"]}}})))
    out.append(case("Yeniden üret: değişen bir şey yok", [fail_step(
        op({"op": "regenerate", "point": "kn-1"}), "nothing_to_set", "Değişecek bir şey yok.", "operation")]))
    ep = changed(base, lambda m: S(m)["campaigns"][0].update(epoch=2026.85))
    out.append(case("Epok kararı 2026,85, hız ızgarası yok: yeniden üretilmez, zincirin reddi; eski sonuç yerinde", [
        *write_step(base, ep, op({"op": "setCampaign", "campaign": "kp-1", "epoch": 2026.85}), {"id": "kp-1"},
                    "Arazi ölçümünü değiştir", then_undo=False),
        fail_step(op({"op": "regenerate", "point": "kn-1"}), "epoch_unsupported",
                  "Koordinatlar 2005 epokundan 2026.85 epokuna taşınmalı: bunun için hız modeli gerekir. Hız ızgarasını "
                  "(doğu, kuzey, yukarı mm/yıl; GeoTIFF) Proje ayarları › Izgaralar'a ekleyip velocity ile verin; "
                  "koordinatlar epok farkı yok sayılarak çevrilmez.", "operation", after_write=True)]))
    epv = changed(base, lambda m: S(m)["campaigns"][0].update(epoch=2026.85, velocity=vel))
    rv = epv.clone()
    pv = rv.survey["points"][0]
    r3 = rv.compute("N1", ["cz-1", "cz-2"], "mean")
    r3["revision"] = 2
    r3["reasons"] = epv.point_why(epv.survey["points"][0])
    pv["history"] = [copy.deepcopy(pv["result"])]
    pv["result"] = r3
    ev = point_entity("gnss", r3, "N1", "kn-1")
    rv.entities[1] = ev
    out.append(case("Epok kararı 2026,85 hız ızgarasıyla: yeniden üretilen nokta 21,85 yıl taşınır, zincir sınırlı", [
        *write_step(base, epv, op({"op": "setCampaign", "campaign": "kp-1", "epoch": 2026.85, "velocity": vel}),
                    {"id": "kp-1"}, "Arazi ölçümünü değiştir", then_undo=False),
        *write_step(epv, rv, op({"op": "regenerate", "point": "kn-1"}), {"id": "kn-1", "uid": P1, "pointRevision": 2},
                    "Noktayı yeniden üret", then_undo=False,
                    extra={"fields": {"1": {"p": ev["p"], "z": ev["z"]}}})]))
    out.append(case("Yeniden üret: nesnesi silinmiş nokta", [
        step("execute", {"uids": [P1]}, "$any", command="cad.entities.delete"),
        fail_step(op({"op": "regenerate", "point": "kn-1"}), "entity_not_found",
                  "“kn-1” noktasının nesnesi çizimde yok; geri alın ya da kaydı çıkarıp noktayı yeniden kabul edin.",
                  "operation.point", after_write=True)]))
    locked = setup(locked_gnss=True)
    locked.survey["sessions"][1]["antenna"]["height"] = 1.81
    out.append(case("Yeniden üret: noktanın katmanı kilitli", [fail_step(
        op({"op": "regenerate", "point": "kn-1"}), "layer_locked",
        "“GNSS noktaları” katmanı kilitli: nokta yeni yerine taşınamaz. Kilidi açın.", "operation.point")], d=locked))
    out.append(case("Yeniden üret: olmayan nokta", [fail_step(
        op({"op": "regenerate", "point": "kn-5"}), "unknown_survey_record",
        "GNSS arşivinde “kn-5” kimlikli kabul edilen nokta yok.", "operation.point")]))
    # use
    after = changed(base, lambda m: S(m)["uses"].append({"id": "kb-2", "operation": "Eş yükselti eğrileri",
                                                         "uids": [F1], "points": [{"point": "kn-1", "revision": 1}]}))
    out.append(case("Dayanan: çizgi kn-1'in 1. revizyonundan", write_step(
        base, after, op({"op": "use", "uids": [F1, F1], "points": ["kn-1"], "operation": "Eş yükselti eğrileri"}),
        {"id": "kb-2"}, "Dayanan kaydı")))
    out.append(case("Dayanan: nesne başka kayıtta", [fail_step(
        op({"op": "use", "uids": [S1], "points": ["kn-1"], "operation": "Yüzey"}), "survey_in_use",
        f"“{S1}” nesnesi “kb-1” dayanan kaydında; önce o kaydı çıkarın.", "operation.uids[0]")]))
    out.append(case("Dayanan: nesne yok", [fail_step(
        op({"op": "use", "uids": [], "points": ["kn-1"], "operation": "Yüzey"}), "no_entities",
        "Dayanan nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "operation.uids")]))
    out.append(case("Dayanan: kimlik UUID değil", [fail_step(
        op({"op": "use", "uids": ["S1"], "points": ["kn-1"], "operation": "Yüzey"}), "invalid_uid",
        "“S1” geçerli bir nesne kimliği değil.", "operation.uids[0]")]))
    out.append(case("Dayanan: çizimde olmayan nesne", [fail_step(
        op({"op": "use", "uids": [GHOST], "points": ["kn-1"], "operation": "Yüzey"}), "entity_not_found",
        f"“{GHOST}” kimlikli nesne çizimde yok.", "operation.uids[0]")]))
    out.append(case("Dayanan: olmayan nokta", [fail_step(
        op({"op": "use", "uids": [F1], "points": ["kn-3"], "operation": "Yüzey"}), "unknown_survey_record",
        "GNSS arşivinde “kn-3” kimlikli kabul edilen nokta yok.", "operation.points[0]")]))
    # remove
    after = changed(base, lambda m: S(m)["solutions"].pop(3))
    out.append(case("Çıkar: kullanılmayan dışlanmış çözüm", write_step(
        base, after, op({"op": "remove", "id": "cz-4"}), {"id": "cz-4", "removed": 1}, "Arşivden çıkar")))
    out.append(case("Çıkar: kabul edilen noktanın çözümü çıkmaz", [fail_step(
        op({"op": "remove", "id": "cz-1"}), "survey_in_use",
        "“cz-1” çözümü “kn-1” noktasında; önce noktayı çıkarın ya da yeniden üretin.", "operation.id")]))
    out.append(case("Çıkar: yüzeyin dayandığı nokta çıkmaz", [fail_step(
        op({"op": "remove", "id": "kn-1"}), "survey_in_use",
        "“kn-1” noktasına “kb-1” dayanan kaydı dayanıyor; önce onu çıkarın.", "operation.id")]))
    no_use = changed(base, lambda m: S(m)["uses"].pop(0))
    no_point = changed(no_use, lambda m: S(m)["points"].pop(0))
    out.append(case("Çıkar: dayanan, sonra noktanın kaydı; nokta nesnesi çizimde kalır", [
        *write_step(base, no_use, op({"op": "remove", "id": "kb-1"}), {"id": "kb-1", "removed": 1}, "Arşivden çıkar",
                    then_undo=False),
        *write_step(no_use, no_point, op({"op": "remove", "id": "kn-1"}), {"id": "kn-1", "removed": 1},
                    "Arşivden çıkar", extra={"ids": [1, 2, 3, 4]})]))
    after = changed(base, lambda m: (S(m)["sessions"].pop(2), S(m)["solutions"].pop(2)))
    out.append(case("Çıkar: N2'nin oturumu çözümüyle", write_step(
        base, after, op({"op": "remove", "id": "os-3"}), {"id": "os-3", "removed": 2}, "Arşivden çıkar")))
    out.append(case("Çıkar: arazi ölçümünün çözümü kabul edilen noktada", [fail_step(
        op({"op": "remove", "id": "kp-1"}), "survey_in_use",
        "“kp-1” kaydının “cz-1” çözümü “kn-1” noktasında; önce noktayı çıkarın ya da yeniden üretin.",
        "operation.id")]))
    out.append(case("Çıkar: olmayan kayıt", [fail_step(
        op({"op": "remove", "id": "zz-1"}), "unknown_survey_record", "GNSS arşivinde “zz-1” kimlikli kayıt yok.",
        "operation.id")]))
    # general
    arch = setup(work="architecture", survey=None)
    out.append(case("Harita projesi değil", [edit(op({"op": "campaign", "name": "X"}), NOT_MAP, UNCHANGED)], d=arch))
    out.append(case("Beklenen sürüm geçersiz", [edit(dict(op({"op": "campaign", "name": "X"}), expectedRevision="abc"),
                                                     BAD_REVISION, UNCHANGED)]))
    out.append(case("Beklenen sürüm eski", [edit(dict(op({"op": "campaign", "name": "X"}), expectedRevision="999"),
                                                 CONFLICT, UNCHANGED)]))
    r = base.compute("N2", ["cz-3"], "single")
    plan_after = changed(base, lambda m: m.survey["points"].append({"id": "kn-2", "mark": "N2", "uid": "x",
                                                                    "result": r}))
    out.append(case("Kabulün planı yazmaz", [edit(op({"op": "accept", "mark": "N2", "solutions": ["cz-3"]}),
                                                  done(edited(plan_after, id="kn-2", pointRevision=1)), UNCHANGED,
                                                  op_="plan")]))
    out.append(case("Doğrulama yazmaz", [edit(op({"op": "exclude", "solution": "cz-3", "reason": "Deneme"}),
                                              done(None), UNCHANGED, op_="validate")]))
    return out


# ── Build ────────────────────────────────────────────────────────────────────────────────────────────────────────

ORIGIN = (0.0, 0.0)


def origin():
    t = pyproj.Transformer.from_pipeline("+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step "
                                         "+proj=tmerc +lat_0=0 +lon_0=30 +k=1 +x_0=500000 +y_0=0 +ellps=GRS80")
    x, y = t.transform(30.5206, 39.7767)
    return float(round(x)), float(round(y))


def built():
    global ORIGIN
    ORIGIN = origin()
    base = setup()
    common = "ADR 1340 (MAP-03). Bağımsız başvuru: scripts/fixtures/map_survey_cases.py (kurallar ADR'den; RINEX başlığı " \
             "sütunlarıyla, NMEA gnss_nmea_cases.py ile; projeye taşıma PROJ ile, crs_vertical_cases.py ve " \
             "geoid_cases.py'nin yolu, zincirin kararı crs_chain_rules.py). Projelenmiş x ve y sonuçlarda ve " \
             "nesnelerde 1e-6 m içinde, ayarlarda her değer."
    q = {"format": "kentos.command-cases", "version": 1, "command": "map.survey.query", "commandVersion": 1,
         "title": "GNSS arşivini sorgula: son ölçülen, kabul edilen, dengelemede, dışlanan; güncel olmayan nokta ve "
                  "dayanan; oturumun oynatma özeti", "note": common, "setup": base.document(),
         "cases": query_cases(base)}
    e = {"format": "kentos.command-cases", "version": 1, "command": "map.survey.edit", "commandVersion": 1,
         "title": "GNSS arşivini düzenle: arazi ölçümü, RINEX/NMEA oturumu, alıcı ve işlenmiş çözüm, dışlama, kabul, "
                  "yeniden üretme, dayanan, çıkarma", "note": common, "setup": base.document(),
         "cases": edit_cases(base)}
    # The network's operations (ADR 1740): their own script writes them, here they join the edit's cases. Imported here:
    # it imports this script as a module.
    import map_survey_network_cases as net
    e["cases"] += net.edit_cases()
    # ADR 4120: the known points' own places on the network's archive.
    q["cases"] += net.query_cases()
    q["title"] += "; bilinen noktaların projedeki kendi yerleri"
    e["title"] += "; baz, bilinen nokta ve ağ dengelemesinin kabulü"
    return q, e


def build():
    tmp = Path(tempfile.mkdtemp(prefix="kentos-survey-"))
    for name in gc.FILES:
        shutil.copy(gc.GRIDS / name, tmp / f"{gc.sha(gc.GRIDS / name)}.{gc.ext(name)}")
    pyproj.datadir.append_data_dir(str(tmp))
    try:
        return built()
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main():
    check = "--check" in sys.argv[1:]
    q, e = build()
    outs = {OUT / "map.survey.query.json": json.dumps(q, ensure_ascii=False, indent=1) + "\n",
            OUT / "map.survey.edit.json": json.dumps(e, ensure_ascii=False, indent=1) + "\n"}
    stale = []
    for name, data in WRITTEN.items():
        path = FILES / name
        if check:
            if not path.exists() or path.read_bytes() != data:
                stale.append(path)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    for path, text in outs.items():
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                stale.append(path)
        else:
            path.write_text(text, encoding="utf-8")
    if stale:
        for p in stale:
            print(f"{p.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
        sys.exit(1)
    n = len(q["cases"]) + len(e["cases"])
    print(f"map.survey: {len(q['cases'])} + {len(e['cases'])} = {n} durum" + (" (denetlendi)" if check else ""))


if __name__ == "__main__":
    main()
