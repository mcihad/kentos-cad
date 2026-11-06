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
`above`). Rust compares its reader with it (crates/native/style line types).

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
