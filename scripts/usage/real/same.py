"""Whether an edit's picture taken at once is the picture taken after the wheel went a notch in and back out
(plan/render.md RM-16, docs/adr/9224): `same.py OUT X,Y,W,H NAME …` compares OUT/NAME.png with OUT/NAME-tekerlek.png
over the region (the drawing area, screen pixels) and writes OUT/NAME-fark.png (white: no match within a pixel, grey:
not the same) for each pair that differs.

A pixel matches when the other picture has its colour, each channel within 40, within one pixel of it: a wheel's
round trip may land the view a fraction of a pixel away. Prints each pair's counts; exits 1 when a pair has pixels with
no match. Standard library only (the pictures are 8-bit RGB or RGBA PNGs, as driver.py takes them).
"""
import struct
import sys
import zlib

CHANNEL = 40


def read(path):
    data = open(path, "rb").read()
    i, width, height, kind, idat = 8, 0, 0, 6, b""
    while i < len(data):
        n = struct.unpack(">I", data[i:i + 4])[0]
        tag, chunk = data[i + 4:i + 8], data[i + 8:i + 8 + n]
        i += 12 + n
        if tag == b"IHDR":
            width, height = struct.unpack(">II", chunk[:8])
            kind = chunk[9]
        elif tag == b"IDAT":
            idat += chunk
    bpp = 4 if kind == 6 else 3
    raw, stride, rows, prev, j = zlib.decompress(idat), width * bpp, [], bytearray(width * bpp), 0
    for _ in range(height):
        f = raw[j]
        line = bytearray(raw[j + 1:j + 1 + stride])
        j += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if f == 1:
                line[x] = (line[x] + a) & 255
            elif f == 2:
                line[x] = (line[x] + b) & 255
            elif f == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif f == 4:
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        prev = line
        rows.append([tuple(line[k:k + 3]) for k in range(0, stride, bpp)])
    return width, height, rows


def write(path, width, height, rows):
    raw = b"".join(b"\x00" + bytes(v for p in row for v in (*p, 255)) for row in rows)

    def chunk(tag, body):
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)

    head = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    open(path, "wb").write(
        b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", head) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")
    )


def matched(a, b, width, height, x, y):
    p = a[y][x]
    for v in range(max(y - 1, 0), min(y + 2, height)):
        for u in range(max(x - 1, 0), min(x + 2, width)):
            q = b[v][u]
            if all(abs(s - t) <= CHANNEL for s, t in zip(p, q)):
                return True
    return False


def main():
    out, region, names = sys.argv[1], sys.argv[2], sys.argv[3:]
    x0, y0, w, h = map(int, region.split(","))
    failed = False
    for name in names:
        width, height, a = read(f"{out}/{name}.png")
        _, _, b = read(f"{out}/{name}-tekerlek.png")
        different = unmatched = 0
        diff = [[(0, 0, 0)] * width for _ in range(height)]
        for y in range(y0, min(y0 + h, height)):
            for x in range(x0, min(x0 + w, width)):
                if a[y][x] == b[y][x]:
                    continue
                different += 1
                if matched(a, b, width, height, x, y) and matched(b, a, width, height, x, y):
                    diff[y][x] = (96, 96, 96)
                else:
                    unmatched += 1
                    diff[y][x] = (255, 255, 255)
        print(f"{name}: {w * h} piksel, farklı {different}, eşi olmayan {unmatched}")
        if different:
            write(f"{out}/{name}-fark.png", width, height, diff)
        failed |= unmatched > 0
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
