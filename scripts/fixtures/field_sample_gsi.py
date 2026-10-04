#!/usr/bin/env python3
"""Writes fixtures/field/v1/sample.gsi: a hand-made Leica GSI-16 field book for the Karne editörü's pictures and its flow
test (docs/adr/0169 §6): two stations with their coordinates and instrument heights, a back sight and four targets in
both faces at the first (one over the 5 mm slope tolerance), a single-face target with a code, a reflector height that
changes, and three targets at the second."""
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "field" / "v1" / "sample.gsi"


def w(wi, info, value, width=16):
    sign = "-" if value < 0 else "+"
    return f"{wi}{info}{sign}{abs(value):0{width}d}"


def text(wi, info, s, width=16):
    return f"{wi}{info}+{s:>0{width}}"


def ang(v):  # gon, five decimals
    return round(v * 100000)


def mm10(v):  # metres in tenths of a millimetre
    return round(v * 10000)


def mm(v):
    return round(v * 1000)


lines = []
n = 1


def block(*words):
    global n
    lines.append("*" + " ".join(words) + " ")
    n += 1


def station(name, e, nn, h, hi):
    block(text("11", f"{n:04d}", name), w("84", "..10", mm(e)), w("85", "..10", mm(nn)), w("86", "..10", mm(h)), w("88", "..10", mm(hi)))


def obs(name, hz, v, sd, th=None, code=None):
    words = [text("11", f"{n:04d}", name), w("21", ".322", ang(hz)), w("22", ".322", ang(v)), w("31", "..06", mm10(sd))]
    if th is not None:
        words.append(w("87", "..10", mm(th)))
    if code:
        words.append(text("71", "....", code))
    block(*words)


station("ST1", 412350.000, 4521800.000, 105.200, 1.552)
obs("P2", 0.0012, 99.8765, 245.6780, 1.700)
obs("101", 87.4321, 101.2345, 63.2140, code="BINA")
obs("102", 120.5550, 100.4400, 82.4410)
obs("103", 210.1200, 98.7000, 55.1230, 2.000)
obs("104", 305.0000, 100.0100, 40.0000, code="AGAC")
obs("103", 10.1214, 301.3008, 55.1290)
obs("102", 320.5544, 299.5606, 82.4415)
obs("101", 287.4337, 298.7671, 63.2160)
obs("P2", 200.0026, 300.1251, 245.6760, 1.700)
station("ST2", 412410.512, 4521742.208, 104.875, 1.480)
obs("ST1", 0.0000, 100.3340, 83.3920, 1.700)
obs("201", 45.2210, 99.1200, 27.5530)
obs("202", 133.0870, 100.8750, 31.0040, code="SINIR")
obs("203", 250.4400, 99.9990, 18.2500)
OUT.write_text("\r\n".join(lines) + "\r\n", encoding="ascii")
print(OUT, len(lines), "blok")
