#!/usr/bin/env python3
"""Writes the hand-made sample field books of fixtures/field/v1 from one table of observations (docs/adr/0169 §6): the
Karne editörü's pictures and flow tests read them, and every format's sample must read into the same book.

The book: a two-station traverse, each station with its coordinates and instrument height; at the first a back sight,
four targets and the second station in both faces (one target over the 5 mm slope tolerance), a single-face target with
a code and a reflector height that changes; at the second the first station, three targets and a fore sight the
traverse ends oriented on. Angles in gon, lengths in metres.

- sample.gsi: Leica GSI-16 (GSI ONLINE for Leica TPS and DNA, 2003).
- sample.sdr: Sokkia SDR33, 14-character names (Interfacing with the SOKKIA SDR Electronic Field Book, 1999).
- sample.gt7: Topcon GTS-7 raw (Topcon Link Reference Manual, Appendix C), gon.
- sample-nikon.raw: Nikon RAW V2.00 (Nivo Series Instruction Manual), gon.
- sample.jxl: Trimble JobXML 5.3, the same angles in decimal degrees (a gon is exactly 0.9°).
"""
from decimal import Decimal
from pathlib import Path

DIR = Path(__file__).resolve().parents[2] / "fixtures" / "field" / "v1"

# (station, east, north, height, instrument height) and its observations (target, hz, zenith, slope, target height, code), gon and metres.
BOOK = [
    (
        ("ST1", 412350.000, 4521800.000, 105.200, 1.552),
        [
            ("P2", 0.0012, 99.8765, 245.6780, 1.700, None),
            ("101", 87.4321, 101.2345, 63.2140, None, "BINA"),
            ("102", 120.5550, 100.4400, 82.4410, None, None),
            ("103", 210.1200, 98.7000, 55.1230, 2.000, None),
            ("104", 305.0000, 100.0100, 40.0000, None, "AGAC"),
            ("ST2", 150.2500, 100.1200, 83.3950, 1.700, None),
            ("ST2", 350.2512, 299.8806, 83.3960, None, None),
            ("103", 10.1214, 301.3008, 55.1290, 2.000, None),
            ("102", 320.5544, 299.5606, 82.4415, None, None),
            ("101", 287.4337, 298.7671, 63.2160, None, None),
            ("P2", 200.0026, 300.1251, 245.6760, 1.700, None),
        ],
    ),
    (
        ("ST2", 412410.512, 4521742.208, 104.875, 1.480),
        [
            ("ST1", 0.0000, 100.3340, 83.3920, 1.700, None),
            ("201", 45.2210, 99.1200, 27.5530, None, None),
            ("202", 133.0870, 100.8750, 31.0040, None, "SINIR"),
            ("203", 250.4400, 99.9990, 18.2500, None, None),
            ("P9", 300.0000, 100.0000, 150.0000, None, None),
        ],
    ),
]


def gsi():
    """Leica GSI-16: a block per station (WI 11, 84–86, 88) and per observation (WI 11, 21, 22, 31, 87, 71)."""

    def w(wi, info, value, width=16):
        sign = "-" if value < 0 else "+"
        return f"{wi}{info}{sign}{abs(value):0{width}d}"

    def text(wi, info, s, width=16):
        return f"{wi}{info}+{s:>0{width}}"

    lines = []
    for (name, e, n, h, hi), observations in BOOK:
        at = f"{len(lines) + 1:04d}"
        lines.append(" ".join([text("11", at, name), w("84", "..10", round(e * 1000)), w("85", "..10", round(n * 1000)), w("86", "..10", round(h * 1000)), w("88", "..10", round(hi * 1000))]))
        for target, hz, v, sd, th, code in observations:
            at = f"{len(lines) + 1:04d}"
            words = [text("11", at, target), w("21", ".322", round(hz * 100000)), w("22", ".322", round(v * 100000)), w("31", "..06", round(sd * 10000))]
            if th is not None:
                words.append(w("87", "..10", round(th * 1000)))
            if code:
                words.append(text("71", "....", code))
            lines.append(" ".join(words))
    return "\r\n".join("*" + l + " " for l in lines) + "\r\n"


def sdr():
    """Sokkia SDR33: the header (gon, metres), the job, the instrument (zenith angles), then per station its record
    (02) and per observation the target height when it changes (03) and the observation (09 F1 or F2 by its zenith)."""

    def f(value, width=16):
        s = str(value)
        assert len(s) <= width
        return s.ljust(width)

    lines = [
        "00NM" + f("SDR33 V04-04.02") + "0000" + f("04-Oct-26 10:00") + "211111",
        "10NM" + f("KARNE") + "121111",
        "01NM:" + f("SET") + "000000" + f("SET") + "000000" + "31" + f("0.00000000") * 3,
    ]
    for (name, e, n, h, hi), observations in BOOK:
        lines.append("02TP" + f(name) + f(f"{n:.4f}") + f(f"{e:.4f}") + f(f"{h:.4f}") + f(f"{hi:.4f}") + f(""))
        for target, hz, v, sd, th, code in observations:
            if th is not None:
                lines.append("03NM" + f(f"{th:.8f}"))
            face = "F1" if v < 200 else "F2"
            lines.append("09" + face + f(name) + f(target) + f(f"{sd:.4f}") + f(f"{v:.8f}") + f(f"{hz:.8f}") + f(code or ""))
    return "\r\n".join(l.rstrip() for l in lines) + "\r\n"


def gts7():
    """Topcon GTS-7: the version, units (metres, gon), per station STN and XYZ, per observation SS with the target height
    in force (GTS-7 writes it on every point) and its code, then SD."""
    lines = ["GTS-700", "JOB         KARNE,", "UNITS       M,G"]
    for (name, e, n, h, hi), observations in BOOK:
        lines.append(f"STN         {name},{hi:.3f},")
        lines.append(f"XYZ         {e:.3f},{n:.3f},{h:.3f}")
        th_now = None
        for target, hz, v, sd, th, code in observations:
            th_now = th if th is not None else th_now
            lines.append(f"SS          {target},{th_now:.3f},{code or ''}")
            lines.append(f"SD          {hz:.4f},{v:.4f},{sd:.4f}")
    return "\r\n".join(lines) + "\r\n"


def nikon():
    """Nikon RAW: the download's comments (metres, gon, zeniths), the stations' coordinates (MP), per station ST and per
    observation SS with the target height in force and its code."""
    lines = ["CO,Nikon RAW data format V2.00", "CO,KARNE", "CO,Dist Units: Metres", "CO,Angle Units: Gons", "CO,Zero VA: Zenith"]
    for (name, e, n, h, hi), _ in BOOK:
        lines.append(f"MP,{name},,{n:.3f},{e:.3f},{h:.3f},")
    for (name, e, n, h, hi), observations in BOOK:
        lines.append(f"ST,{name},,{observations[0][0]},,{hi:.3f},0.0000,0.0000")
        th_now = None
        for target, hz, v, sd, th, code in observations:
            th_now = th if th is not None else th_now
            lines.append(f"SS,{target},{th_now:.3f},{sd:.4f},{hz:.4f},{v:.4f},10:00:00,{code or ''}")
    return "\r\n".join(lines) + "\r\n"


def jobxml():
    """Trimble JobXML: per station its coordinates (a keyed-in PointRecord with a Grid) and its StationRecord, a
    TargetRecord whenever the target height changes, per observation a DirectReading PointRecord (or AngleOnly for one
    without a distance) whose Circle holds the readings in decimal degrees, its face by its zenith."""
    deg = lambda v: format(Decimal(repr(v)) * Decimal("0.9"), "f")
    ids = iter(range(1, 1000))
    rows = ['<?xml version="1.0" encoding="UTF-8"?>', '<JOBFile jobName="KARNE" product="Trimble Access" productVersion="2026.00" version="5.3">', "  <FieldBook>"]
    for (name, e, n, h, hi), observations in BOOK:
        rows.append(f'    <PointRecord ID="{next(ids):08d}" TimeStamp="2026-10-04T10:00:00"><Name>{name}</Name><Code></Code><Method>Coordinates</Method><SurveyMethod>KeyedIn</SurveyMethod><Classification>Normal</Classification><Deleted>false</Deleted><Grid><North>{n:.3f}</North><East>{e:.3f}</East><Elevation>{h:.3f}</Elevation></Grid></PointRecord>')
        station = f"{next(ids):08d}"
        rows.append(f'    <StationRecord ID="{station}" TimeStamp="2026-10-04T10:01:00"><StationName>{name}</StationName><TheodoliteHeight>{hi:.3f}</TheodoliteHeight><StationType>StandardStation</StationType></StationRecord>')
        target = None
        for o_target, hz, v, sd, th, code in observations:
            if th is not None or target is None:
                target = f"{next(ids):08d}"
                rows.append(f'    <TargetRecord ID="{target}" TimeStamp="2026-10-04T10:02:00"><PrismConstant>0</PrismConstant><TargetHeight>{th:.3f}</TargetHeight></TargetRecord>')
            face = "Face1" if v < 200 else "Face2"
            rows.append(f'    <PointRecord ID="{next(ids):08d}" TimeStamp="2026-10-04T10:03:00"><Name>{o_target}</Name><Code>{code or ""}</Code><Method>DirectReading</Method><SurveyMethod>Fix</SurveyMethod><Classification>Normal</Classification><Deleted>false</Deleted><Circle><HorizontalCircle>{deg(hz)}</HorizontalCircle><VerticalCircle>{deg(v)}</VerticalCircle><EDMDistance>{sd:.4f}</EDMDistance><Face>{face}</Face></Circle><StationID>{station}</StationID><TargetID>{target}</TargetID></PointRecord>')
    rows += ["  </FieldBook>", "</JOBFile>"]
    return "\n".join(rows) + "\n"


for name, write in (("sample.gsi", gsi), ("sample.sdr", sdr), ("sample.gt7", gts7), ("sample-nikon.raw", nikon), ("sample.jxl", jobxml)):
    text = write()
    (DIR / name).write_text(text, encoding="ascii", newline="")
    print(DIR / name, text.count("\n"), "satır")
