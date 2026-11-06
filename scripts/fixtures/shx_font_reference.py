#!/usr/bin/env python3
"""SHX unifont and Big Font test fonts and their independent reading (ADR 1120).

KentOS reads no third-party font in its tests. This script compiles two small
fonts of its own from shape-language programs (Autodesk's Shape Descriptions
and Special Codes, the Unicode font description and Big Font description) into
`fixtures/shx/v1/`:

- `kentos-uni.shx`: AutoCAD-86 unifont 1.0, Unicode encoding, embedding
  allowed; `kentos-uni-noembed.shx`: the same font with type 1 (not embeddable).
- `kentos-big.shx`: AutoCAD-86 bigfont 1.0, one escape range 81–9F.

and writes `expected.json`: what this script's own reader (no KentOS code)
draws for a few text runs, in capital heights (each glyph scaled by its font's
`above`), and (ADR 1125) for text objects in these fonts placed on the ground
by their alignment, turn, width factor, slant, lines and box. Rust compares its
reader (crates/native/style line types) and its text layout
(crates/native/application `shx_text`) with it.

Layouts (ADR 1120 §2): unifont = signature, u32 record count (with the
definition record), u16 definition size, definition (name NUL above below
modes encoding type 0), then records (u16 number, u16 size, name NUL program).
Big Font = signature, u16 header word (written as 4 + 4·ranges; neither reader
interprets it), u16 index entries, u16 ranges, ranges (u16 first, u16 last),
entries (u16 code, u16 size, u32 absolute offset; all zero is an empty slot),
records. A unifont's subshape operand is two bytes, high first.

    python3 scripts/fixtures/shx_font_reference.py          # write
    python3 scripts/fixtures/shx_font_reference.py --check  # compare, write nothing
"""

import json
import math
import struct
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "shx" / "v1"
UNIFONT = b"AutoCAD-86 unifont 1.0\r\n\x1a"
BIGFONT = b"AutoCAD-86 bigfont 1.0\r\n\x1a"


def pair(x, y):
    return [x & 0xFF, y & 0xFF]


# Programs in vector units, capital height 8. Vector byte: length << 4 | direction
# (0 east, 4 north, 8 west, C south).
S = [0x60, 0x44, 0x68, 0x44, 0x60, 2, 8, *pair(2, -8), 0]
U = [2, 0x84, 1, 9, *pair(0, -8), *pair(6, 0), *pair(0, 8), 0, 0, 2, 8, *pair(2, -8), 0]
S_CEDILLA = [7, 0x00, 0x53, 2, 8, *pair(-5, 0), 1, 0x2C, 2, 8, *pair(5, 2), 0]
DASH = [14, 0x4C, 2, 0x44, 1, 0x60, 2, 8, *pair(2, -4), 0]
SPACE = [2, 0x80, 0]
UNI_GLYPHS = {0x20: SPACE, 0x2D: DASH, 0x53: S, 0x55: U, 0x15E: S_CEDILLA}

TICK = [0x24, 2, 0x2C, 0x30, 0]
BOX = [0x80, 0x84, 0x88, 0x8C, 2, 8, *pair(10, 0), 0]
CROSS = [2, 0x44, 1, 0x80, 2, 8, *pair(-4, 4), 1, 0x8C, 2, 8, *pair(6, 0), 0]
TICKED = [7, 0x01, 0x70, 0]
BIG_GLYPHS = {1: TICK, 0x8140: BOX, 0x8141: CROSS, 0x8142: TICKED}


def unifont(embed):
    definition = b"KentOS Test Unifont\0" + bytes([8, 2, 0, 0, embed, 0])
    out = bytearray(UNIFONT)
    out += struct.pack("<IH", len(UNI_GLYPHS) + 1, len(definition)) + definition
    for number, code in sorted(UNI_GLYPHS.items()):
        record = b"\0" + bytes(code)
        out += struct.pack("<HH", number, len(record)) + record
    return bytes(out)


def bigfont():
    ranges = [(0x81, 0x9F)]
    records = [(0, b"KentOS Test Big Font\0" + bytes([8, 2, 0, 0]))]
    records += [(n, b"\0" + bytes(c)) for n, c in sorted(BIG_GLYPHS.items())]
    out = bytearray(BIGFONT)
    entries = len(records) + 1
    out += struct.pack("<HHH", 4 + 4 * len(ranges), entries, len(ranges))
    for first, last in ranges:
        out += struct.pack("<HH", first, last)
    offset = len(out) + 8 * entries
    index = bytearray(8)  # an empty slot first
    for number, record in records:
        index += struct.pack("<HHI", number, len(record), offset)
        offset += len(record)
    out += index
    for _, record in records:
        out += record
    return bytes(out)


# ---------------------------------------------------------------- the reader
def read(data):
    """A font's form, descriptor and programs, from the bytes alone."""
    if data.startswith(UNIFONT):
        at = len(UNIFONT)
        count, size = struct.unpack_from("<IH", data, at)
        at += 6
        definition = data[at:at + size]
        at += size
        rest = definition[definition.index(0) + 1:]
        assert len(rest) == 6 and rest[5] == 0, "unifont definition"
        font = {"kind": "unifont", "above": rest[0], "encoding": rest[3], "type": rest[4], "programs": {}}
        for _ in range(count - 1):
            number, size = struct.unpack_from("<HH", data, at)
            record = data[at + 4:at + 4 + size]
            assert len(record) == size, "unifont record cut"
            font["programs"][number] = record[record.index(0) + 1:]
            at += 4 + size
        assert at == len(data), "unifont trailer"
        return font
    if data.startswith(BIGFONT):
        at = len(BIGFONT)
        _, entries, nranges = struct.unpack_from("<HHH", data, at)
        at += 6
        ranges = [struct.unpack_from("<HH", data, at + 4 * i) for i in range(nranges)]
        at += 4 * nranges
        font = {"kind": "bigfont", "ranges": ranges, "programs": {}}
        for i in range(entries):
            number, size, offset = struct.unpack_from("<HHI", data, at + 8 * i)
            if (number, size, offset) == (0, 0, 0):
                continue
            record = data[offset:offset + size]
            assert len(record) == size, "bigfont record outside the file"
            program = record[record.index(0) + 1:]
            if number == 0:
                font["above"] = program[0]
            else:
                font["programs"][number] = program
        return font
    raise ValueError("not a unifont or Big Font")


DX = [1, 1, 1, 0.5, 0, -0.5, -1, -1, -1, -1, -1, -0.5, 0, 0.5, 1, 1]
DY = [0, 0.5, 1, 1, 1, 1, 1, 0.5, 0, -0.5, -1, -1, -1, -1, -1, -0.5]


def signed(b):
    return b - 256 if b > 127 else b


class Pen:
    def __init__(self):
        self.at = (0.0, 0.0)
        self.down = True
        self.scale = 1.0
        self.paths = []
        self.path = []

    def flush(self):
        if len(self.path) >= 2:
            self.paths.append(self.path)
        self.path = []

    def move(self, dx, dy):
        to = (self.at[0] + self.scale * dx, self.at[1] + self.scale * dy)
        if self.down:
            if not self.path:
                self.path = [self.at]
            self.path.append(to)
        else:
            self.flush()
        self.at = to

    def run(self, font, number):
        code = font["programs"][number]
        i = 0
        skip = False
        while True:
            op = code[i]
            i += 1
            apply = not skip
            skip = False
            if op == 0:
                return
            if op == 1:
                if apply:
                    self.down = True
            elif op == 2:
                if apply:
                    self.flush()
                    self.down = False
            elif op == 7:
                if font["kind"] == "unifont":
                    sub = code[i] << 8 | code[i + 1]
                    i += 2
                else:
                    sub = code[i]
                    i += 1
                if apply:
                    self.run(font, sub)
            elif op == 8:
                dx, dy = signed(code[i]), signed(code[i + 1])
                i += 2
                if apply:
                    self.move(dx, dy)
            elif op == 9:
                while True:
                    dx, dy = signed(code[i]), signed(code[i + 1])
                    i += 2
                    if dx == 0 and dy == 0:
                        break
                    if apply:
                        self.move(dx, dy)
            elif op == 14:
                skip = True
            elif op >= 0x10:
                if apply:
                    n = op >> 4
                    self.move(DX[op & 15] * n, DY[op & 15] * n)
            else:
                raise ValueError(f"code {op} is not used by these fonts")


def draw(run):
    """(font, number) glyphs, each from the pen's place, its font's scale, pen down."""
    pen = Pen()
    for font, number in run:
        pen.flush()
        pen.down = True
        pen.scale = 1.0 / font["above"]
        pen.run(font, number)
    pen.flush()
    return [[list(p) for p in path] for path in pen.paths]


def runs(uni, big):
    def big_code(code):
        lead = code >> 8
        assert any(a <= lead <= b for a, b in big["ranges"]), "lead byte outside the escape ranges"
        return big, code

    cases = {
        "ŞU": [(uni, 0x15E), (uni, 0x55)],
        "S-U S": [(uni, 0x53), (uni, 0x2D), (uni, 0x55), (uni, 0x20), (uni, 0x53)],
        "Ş\\M+18140\\M+18141\\M+18142U": [(uni, 0x15E), big_code(0x8140), big_code(0x8141), big_code(0x8142),
                                       (uni, 0x55)],
    }
    return {text: draw(run) for text, run in cases.items()}


def draw_advance(run):
    """`draw` and where the pen ends: the run's advance in capital heights."""
    pen = Pen()
    for font, number in run:
        pen.flush()
        pen.down = True
        pen.scale = 1.0 / font["above"]
        pen.run(font, number)
    pen.flush()
    return [[list(p) for p in path] for path in pen.paths], pen.at[0]


def chars(uni, big, text):
    """A line's glyphs: `\\M+cXXXX` (c 1–5) a Big Font code, any other character the unifont's own number."""
    out, i = [], 0
    while i < len(text):
        w = text[i:i + 8]
        if len(w) == 8 and w[:3].lower() == "\\m+" and w[3] in "12345" and all(c in "0123456789abcdefABCDEF" for c in w[4:]):
            code = int(w[4:], 16)
            assert any(a <= code >> 8 <= b for a, b in big["ranges"]) and code in big["programs"]
            out.append((big, code))
            i += 8
        else:
            assert ord(text[i]) in uni["programs"], text[i]
            out.append((uni, ord(text[i])))
            i += 1
    return out


# ADR 0145 §1: where `p` is along a text (a share of its box's width) and up from its first baseline (of its height).
ALONG = {"baselineCenter": 0.5, "baselineRight": 1, "bottomLeft": 0, "bottomCenter": 0.5, "bottomRight": 1,
         "middleLeft": 0, "middleCenter": 0.5, "middleRight": 1, "topLeft": 0, "topCenter": 0.5, "topRight": 1}
UP = {"baselineCenter": 0, "baselineRight": 0, "bottomLeft": -0.2, "bottomCenter": -0.2, "bottomRight": -0.2,
      "middleLeft": 0.5, "middleCenter": 0.5, "middleRight": 0.5, "topLeft": 1, "topCenter": 1, "topRight": 1}


def placed(uni, big, t):
    """A text in the SHX fonts on the ground (ADR 1125 §2), by the rules of ADR 0145 and 0182, no KentOS code: its
    breaks, then (in a box) a word put on the next line when the line up to it would pass the box (the first line
    from the paragraph's start, each next from its word; a line's trailing spaces take no room); lines 5/3 of the
    height times the spacing apart; each line along its box by the alignment's share; `p` the alignment's point;
    the slant leaning the text's own frame."""
    h, wf = t["height"], t.get("widthFactor", 1.0)
    k = math.tan(math.radians(t.get("oblique", 0.0)))
    box = t.get("boxWidth")

    def run(s):
        return draw_advance(chars(uni, big, s))

    lines = []
    for para in t["text"].split("\n"):
        para = para[:-1] if para.endswith("\r") else para
        if box is None:
            lines.append(run(para.rstrip(" ")))
            continue
        start, filled, i = 0, None, 0
        while True:
            while i < len(para) and para[i] == " ":
                i += 1
            if i >= len(para):
                break
            word = i
            while i < len(para) and para[i] != " ":
                i += 1
            if filled is not None and run(para[start:i])[1] * h * wf > box:
                lines.append(run(para[start:filled].rstrip(" ")))
                start = word
            filled = i
        lines.append(run(para[start:].rstrip(" ")))
    widths = [adv * h * wf for _, adv in lines]
    width = box if box is not None else max(widths)
    pitch = 5.0 / 3.0 * h * t.get("lineSpacing", 1.0)
    n = len(lines)
    align = t.get("align")
    along = ALONG[align] if align else 0.0
    up = UP[align] if align else 0.0
    below = (n - 1) * pitch
    rise = up * h - below if up < 0 else (up * h - below / 2 if 0 < up < 1 else up * h)
    r = math.radians(t["rotation"])
    u, v = (math.cos(r), math.sin(r)), (-math.sin(r), math.cos(r))
    px, py = t["p"]["x"], t["p"]["y"]
    ox = px - u[0] * along * width - v[0] * rise
    oy = py - u[1] * along * width - v[1] * rise

    def at(x, y):
        x = x + y * k
        return [ox + u[0] * x + v[0] * y, oy + u[1] * x + v[1] * y]

    paths = []
    for i, ((strokes, _), w) in enumerate(zip(lines, widths)):
        shift = along * (width - w)
        for path in strokes:
            paths.append([at(shift + q[0] * h * wf, -i * pitch + q[1] * h) for q in path])
    return {"width": width, "paths": paths}


def texts(uni, big):
    """The shared cases of an SHX text's place (ADR 1125 §6): each the text object's fields and what it draws."""
    base = {"p": {"x": 10.0, "y": 20.0}, "height": 2.5, "rotation": 0.0}
    cases = [
        ("Sol taban", {"text": "SU"}),
        ("30° döndürülmüş", {"text": "SU", "rotation": 30.0}),
        ("−90° döndürülmüş", {"text": "S-U", "rotation": -90.0}),
        ("Taban ortası", {"text": "SU", "align": "baselineCenter"}),
        ("Taban sağı", {"text": "S U", "align": "baselineRight"}),
        ("Orta orta", {"text": "SU", "align": "middleCenter"}),
        ("Üst sol", {"text": "US", "align": "topLeft"}),
        ("Alt sağ", {"text": "SUS", "align": "bottomRight"}),
        ("Genişlik çarpanı 0,8", {"text": "SU", "widthFactor": 0.8}),
        ("Eğiklik 15°", {"text": "SU", "oblique": 15.0}),
        ("Çarpan, eğiklik ve dönüş birlikte", {"text": "Ş-U", "widthFactor": 1.25, "oblique": -10.0, "rotation": 45.0,
                                               "align": "middleRight"}),
        ("İki satır", {"text": "SU\nUS"}),
        ("İki satır orta orta", {"text": "SU\nS", "align": "middleCenter"}),
        ("Üç satır, aralık 1,5, üst sağ", {"text": "S\nSU\nSUS", "lineSpacing": 1.5, "align": "topRight"}),
        ("Kutuya sözcük kaydırma", {"text": "S U SU US", "boxWidth": 4.5}),
        ("Kutuya kaydırma, ortalı", {"text": "SU US S", "boxWidth": 5.0, "align": "topCenter"}),
        ("Big Font karakteri", {"text": "Ş\\M+18140U"}),
        ("Sondaki boşluk yer tutmaz (sağa hizalı)", {"text": "SU  ", "align": "baselineRight"}),
        ("Kutuda baştaki boşluk ilk satırda kalır", {"text": "  S U", "boxWidth": 2.0}),
        ("Büyük koordinat", {"text": "SU", "p": {"x": 500123.25, "y": 4400456.75}, "rotation": 12.5}),
        ("Boş satır", {"text": "S\n\nU", "align": "bottomLeft"}),
        ("Küçük yazı ters dönmüş", {"text": "US-S", "height": 0.1, "rotation": 180.0, "align": "bottomCenter"}),
        ("Satır sonu CR LF", {"text": "S\r\nU"}),
    ]
    out = []
    for name, fields in cases:
        t = dict(base, **fields)
        out.append({"name": name, "text": t, **placed(uni, big, t)})
    return out


def main():
    files = {
        "kentos-uni.shx": unifont(0),
        "kentos-uni-noembed.shx": unifont(1),
        "kentos-big.shx": bigfont(),
    }
    uni = read(files["kentos-uni.shx"])
    big = read(files["kentos-big.shx"])
    assert read(files["kentos-uni-noembed.shx"])["type"] == 1
    expected = {
        "note": "scripts/fixtures/shx_font_reference.py: text runs in capital heights (ADR 1120)",
        "runs": runs(uni, big),
        "texts": texts(uni, big),
    }
    files["expected.json"] = (json.dumps(expected, ensure_ascii=False, indent=1) + "\n").encode()
    check = "--check" in sys.argv[1:]
    bad = 0
    for name, data in files.items():
        path = OUT / name
        if check:
            if not path.exists() or path.read_bytes() != data:
                print(f"farklı: {path}")
                bad += 1
        else:
            OUT.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    print(f"{len(files)} dosya {'denetlendi' if check else 'yazıldı'}; {bad} fark")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
