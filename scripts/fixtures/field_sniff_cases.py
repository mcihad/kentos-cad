#!/usr/bin/env python3
"""Independent reference of how a field book's format is told by its content (docs/adr/0169 §1, §6; steps 3b, 5).

Writes fixtures/field/v1/sniff.json from the rules alone, no KentOS code: which files Karne editörü reads as Leica GSI or
Sokkia SDR and which as a text book whose columns the user maps.

The rules:

1. A byte order mark is not text. Lines end with LF, CR LF or CR; blank lines are passed over.
2. The first line that is not blank decides: without a leading "*", its first word (up to a blank) is a GSI word when it
   has at least eight characters, its first two are ASCII digits, its next four ASCII digits or dots, and its seventh
   is "+" or "-". Then the book is GSI.
3. Otherwise the book is Sokkia SDR when one of its first five lines that are neither blank nor a transmission's frame
   (beginning with STX, hex 02, or ETX, hex 03) is an SDR header: its first seven characters "00", two capital letters
   A–Z (the derivation code) and "SDR" (records before it are said by the reader). Otherwise (or with no line at all)
   the book is a text book.
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "field" / "v1" / "sniff.json"
DIGITS = "0123456789"


def sniff(text):
    if text.startswith("﻿"):
        text = text[1:]
    lines = [raw.strip() for raw in text.replace("\r\n", "\n").replace("\r", "\n").split("\n")]
    lines = [line for line in lines if line]
    if not lines:
        return None
    first = lines[0][1:] if lines[0].startswith("*") else lines[0]
    words = first.split()
    w = words[0] if words else ""
    if len(w) >= 8 and all(c in DIGITS for c in w[:2]) and all(c in DIGITS or c == "." for c in w[2:6]) and w[6] in "+-":
        return "gsi"
    records = [line for line in lines if line[0] not in "\x02\x03"][:5]
    if any(len(h) >= 7 and h[:2] == "00" and all("A" <= c <= "Z" for c in h[2:4]) and h[4:7] == "SDR" for h in records):
        return "sdr"
    return None


def cases():
    gsi = json.loads((ROOT / "fixtures" / "field" / "v1" / "gsi.json").read_text(encoding="utf-8"))
    sdr = json.loads((ROOT / "fixtures" / "field" / "v1" / "sdr.json").read_text(encoding="utf-8"))
    return [
        ("GSI-8", gsi["cases"][0]["text"]),
        ("GSI-16, CR LF", gsi["cases"][1]["text"]),
        ("BOM ve boş satırlardan sonra GSI", "﻿\n\n   110001+00000P01 21.322+00000000\n"),
        ("kod bloğuyla başlayan GSI", "410001+0000BINA 42....+00000001\n110002+00000P02 21.322+00000000\n"),
        ("eksi işaretli ilk sözcük", "860001-00001000 88..10+00001500\n"),
        ("CR ile biten satırlar", "\r\r110001+00000P01 21.322+00000000\r"),
        ("noktalı virgüllü karne, başlıklı", "İstasyon;Nokta;Hz;V;SD\nS1;P1;0;100;12.3\n"),
        ("sayılarla başlayan karne", "1;2;3;4;5;6;7;8\n"),
        ("sekmeyle ayrılmış karne", "S1\tP1\t0.0000\t100.0000\n"),
        ("işaretsiz on dört rakam", "11000100000101\n"),
        ("kısa sözcük", "110001+\n"),
        ("yalnız yıldız", "*\n110001+00000P01\n"),
        ("boş dosya", ""),
        ("SDR33, STX ile", sdr["cases"][0]["text"]),
        ("SDR2x", sdr["cases"][1]["text"]),
        ("SDR'ye benzemeyen başlık", "00NMXYZ33 V04-04.02\n"),
        ("küçük harfli türetme kodu", "00nmSDR33 V04-04.02\n"),
        ("STX'ten sonra metin karne", "\x02\nİstasyon;Nokta;Hz\n"),
        ("başlıktan önce kayıt", "09F1000110011234.56789\n00NMSDR20 V03-05\n"),
        ("beşinci satırdan sonra başlık", "a\nb\nc\nd\ne\n00NMSDR20 V03-05\n"),
    ]


def build():
    return {
        "format": "kentos.field-sniff",
        "version": 1,
        "source": "scripts/fixtures/field_sniff_cases.py (docs/adr/0169 §1, §6)",
        "cases": [{"name": name, "text": text, "format": sniff(text)} for name, text in cases()],
    }


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
