#!/usr/bin/env python3
# Writes the Netcad NCZ fixtures under fixtures/formats/v1/ncz/: the synthetic
# drawings the NCZ reader (crates/shared/ncz) is tested with, natively and in
# WASM (docs/adr/0138).
#
# WHY A GENERATOR AND NOT JUST THE FILES. An NCZ is binary and nothing in a diff
# of one says what it holds. Every block below is built from its offsets, named
# the way the reader names them, so a reviewer reads WHAT a fixture holds here
# and the bytes are only the consequence. Running this again writes the same
# bytes: nothing here is random, and the float values are chosen to be exact.
#
# No real drawing is in the fixtures: a user's NCZ is their municipality's data.
# The layout below is the one the NCZ Reader plugin reads (and, for Netcad 8's
# smart objects, what their records were found to hold); that is all a fixture
# needs.
#
#   python3 scripts/fixtures/ncz_reference.py            # (re)write the fixtures
#   python3 scripts/fixtures/ncz_reference.py --check    # fail if they differ
import math
import pathlib
import struct
import sys

EXT = 28  # the extended header of a type-22 geometry block


def u32(v):
    return struct.pack('<I', v)


def f64(v):
    return struct.pack('<d', v)


def f32(v):
    return struct.pack('<f', v)


def block(kind, body):
    """One top-level block: its type byte, the length of what follows, the rest."""
    return bytes([kind]) + u32(len(body)) + bytes(body)


def legacy(text):
    """Text in the legacy code page the reader decodes: Latin-1 plus the six
    Turkish letters of Windows-1254."""
    table = {'İ': 221, 'Ş': 222, 'Ğ': 208, 'ğ': 240, 'ı': 253, 'ş': 254}
    return bytes(table.get(ch, ord(ch)) for ch in text)


class Geometry:
    """A geometry block of `total` bytes, written by absolute offset."""

    def __init__(self, gtype, total, extended=False, layer=0, color=0):
        self.buf = bytearray(total)
        self.buf[0] = 22 if extended else 21
        self.buf[1:5] = u32(total - 5)
        self.buf[5] = gtype  # the byte an embedded scan matches against the type
        self.buf[6] = gtype
        self.buf[7] = layer
        self.buf[37] = color
        self.ext = EXT if extended else 0

    def put(self, at, data):
        self.buf[at:at + len(data)] = data
        return self

    def xy(self, north, east, at=8):
        """The file's order: northing (the Turkish X) first."""
        return self.put(at, f64(north)).put(at + 8, f64(east))

    def bytes(self):
        return bytes(self.buf)


def point(north, east, name, z=0.0, **kw):
    g = Geometry(1, 87 + EXT * kw.get('extended', False) + len(name) + 3, **kw)
    g.xy(north, east).put(24, f32(z))
    g.put(g.ext + 86, bytes([len(legacy(name))])).put(g.ext + 87, legacy(name))
    return g.bytes()


def line(a, b, total=48, **kw):
    g = Geometry(2, total, **kw)
    g.xy(*a).put(24, f32(0.0))
    block_size = total - 1
    g.put(block_size - 19, f64(b[0])).put(block_size - 11, f64(b[1])).put(block_size - 3, f32(0.0))
    return g.bytes()


def circle(centre, radius, **kw):
    g = Geometry(3, 80, **kw)
    g.xy(*centre).put(50, f64(centre[0] + radius)).put(66, f64(centre[0] - radius))
    return g.bytes()


def arc(centre, radius, start, end, **kw):
    g = Geometry(4, 120 + EXT * kw.get('extended', False) + 4, **kw)
    g.xy(*centre)
    g.put(g.ext + 86, f64(radius)).put(g.ext + 104, f64(start)).put(g.ext + 112, f64(end))
    return g.bytes()


def text(at, words, height, rotation_rad=0.0, **kw):
    data = legacy(words)
    g = Geometry(5, 98 + EXT * kw.get('extended', False) + len(data) + 2, **kw)
    g.xy(*at).put(g.ext + 86, f32(height)).put(g.ext + 90, f32(rotation_rad))
    g.put(g.ext + 97, bytes([len(data)])).put(g.ext + 98, data)
    return g.bytes()


def symbol(at, code, size, rotation_rad=0.0, **kw):
    g = Geometry(6, 96 + EXT * kw.get('extended', False), **kw)
    g.xy(*at).put(g.ext + 86, f32(size)).put(g.ext + 90, f32(rotation_rad))
    g.put(g.ext + 94, bytes([code]))
    return g.bytes()


def multiline(points, label='', **kw):
    ext = EXT if kw.get('extended', False) else 0
    g = Geometry(7, 113 + ext + 24 * len(points), **kw)
    data = legacy(label)
    g.put(g.ext + 86, bytes([len(data)])).put(g.ext + 87, data)
    for i, (north, east) in enumerate(points):
        g.put(g.ext + 113 + 24 * i, f64(north) + f64(east) + f64(0.0))
    return g.bytes()


def compressed(origin, deltas, **kw):
    ext = EXT if kw.get('extended', False) else 0
    g = Geometry(9, 122 + ext + 18 * len(deltas) + 1, **kw)
    g.xy(*origin)
    for i, (dn, de) in enumerate(deltas):
        g.put(g.ext + 122 + 18 * i, f32(dn) + f32(de))
    return g.bytes()


def box(corner, far, rotation_rad, name='', **kw):
    data = name.encode('ascii')
    g = Geometry(10, 124 + EXT * kw.get('extended', False) + 8 + len(data), **kw)
    g.xy(*corner).put(g.ext + 104, f64(far[0])).put(g.ext + 112, f64(far[1]))
    g.put(g.ext + 120, f32(rotation_rad))
    g.put(g.ext + 126, data)
    return g.bytes()


def map_sheet(a, b, name, **kw):
    data = legacy(name)
    g = Geometry(11, 88 + len(data) + 2, **kw)
    g.put(50, f64(a[0]) + f64(a[1]) + f64(b[0]) + f64(b[1]))
    g.put(86, bytes([len(data)])).put(87, data)
    return g.bytes()


def triangle(a, b, c, **kw):
    g = Geometry(12, 122, **kw)
    g.xy(*a).put(24, f32(0.0)).xy(*b, at=86).xy(*c, at=106)
    return g.bytes()


def block_ref(at, name, rotation_rad=0.0, **kw):
    ext = EXT if kw.get('extended', False) else 0
    data = legacy(name)
    g = Geometry(13, 122 + ext + len(data) + 2, **kw)
    g.xy(*at).put(g.ext + 86, bytes([len(data)])).put(g.ext + 87, data)
    g.put(g.ext + 118, f32(rotation_rad))
    return g.bytes()


def smart(corner, width, height, grads, scale=500.0, token='BASIC', **kw):
    g = Geometry(15, 201, **kw)
    g.xy(*corner).put(82, f32(grads)).put(86, f32(scale))
    g.put(145, token.encode('ascii'))
    g.put(169, f64(width) + f64(height) + f64(10.0) + f64(10.0))
    return g.bytes()


# Netcad 8 SmartObjects ("akıllı nesne"): the Planet symbols, with the property
# block Netcad writes after the record — which runs 81 bytes past the size the
# record declares, as every one in a real plan does.
GUIDS = {
    'yol': '89383f961207d34fba5a4e5f523cf239',
    'yerlesim': '7b865f8e160c474cbc7b523f411216df',
    'yapilasma': '3ed066ec376b7041ad0eebca2d350e10',
    'plannotu': 'f86f2b1e0c328441843d2c566b316d5c',
    'fonksiyon': '89897e489d60c84388835aaf333c3074',
    'bilinmeyen': '00112233445566778899aabbccddeeff',
}


def varint(n):
    out = bytearray()
    while True:
        b = n & 0x7F
        n >>= 7
        out.append(b | (0x80 if n else 0))
        if not n:
            return bytes(out)


def prop(name, value, display, user=True, kind=0x0f):
    data = value.encode('utf-8')
    label = display.encode('utf-8')
    return (varint(len(name)) + name.encode('ascii') + bytes([kind]) + varint(len(data)) + data +
            varint(len(label)) + label + bytes([1, 0, 0, 0, 0, 0, 1 if user else 0]))


def planet(cls, at, props, size=0.5, grads=0.0, layer=0, broken=False):
    guid = bytes.fromhex(GUIDS[cls])
    body = bytearray()
    for p in props:
        body += p
    count = len(props)
    # A real record never declares fewer than 124 bytes: pad a small one with the
    # setting every plan-note record carries, which draws nothing.
    while 118 + 25 + len(body) + 4 - 81 < 124:
        body += prop('imageDrawMode', '0', 'imageDrawMode', False, 0x07)
        count += 1
    count += 5000 if broken else 0
    payload = bytes([1]) + guid + b'\xff\xff\x0f\x10' + struct.pack('<I', count) + bytes(body)
    payload += bytes(4)  # the list ends four bytes before the record does
    total = 118 + len(payload)
    g = Geometry(15, total, layer=layer)
    g.xy(*at)
    r = 10.0 * size
    g.put(50, f64(at[0] - r) + f64(at[1] - r) + f64(at[0] + r) + f64(at[1] + r))
    g.put(82, f32(grads)).put(86, f32(size))
    g.put(94, guid).put(110, struct.pack('<i', -2)).put(114, struct.pack('<I', total - 110))
    g.put(118, payload)
    g.buf[1:5] = u32(total - 81 - 5)  # declared 81 bytes short, as Netcad writes it
    return g.bytes()


def rtf_note():
    rtf = (r'{\rtf1\ansi\ansicpg1254{\fonttbl{\f0 Arial;}}{\colortbl;\red0\green0\blue0;}'
           r'\f0\fs20 PLAN NOTLARI\par 1. Bu alanda \u304\'ddmar Kanunu uygulan\u305\'fdr.\par '
           r'2. Kot k\'f6\u351\'feeden al\u305\'fdn\u305\'fdr.}')
    import base64
    return base64.b64encode(rtf.encode('ascii')).decode('ascii')


def unknown(gtype):
    return Geometry(gtype, 64).bytes()


def version(name):
    data = legacy(name)
    return block(25, bytes([len(data)]) + data)


def named(name, rest):
    data = legacy(name)
    return block(28, bytes([len(data)]) + data + rest)


def mproj(projection, datum, zone):
    # The name ends at +6+len; the reader looks at +16, +17 and +21 absolutely.
    body = bytearray(bytes([5]) + b'MPROJ' + bytes(20))
    body[16 - 5] = projection
    body[17 - 5] = datum
    body[21 - 5] = zone
    return block(28, bytes(body))


def tiled_xml(srs):
    return named('TILED_XML', b'<TILE ' + srs.encode('ascii') + b'>')


def lex_colours(colours):
    body = bytearray(bytes([7]) + b'LEX.ST2' + bytes(79 - 13 + 256 * len(colours)))
    body[20 - 5] = len(colours)
    for i, (r, g, b) in enumerate(colours):
        at = 79 + 256 * i - 5
        body[at:at + 3] = bytes([r, g, b])
    return block(28, bytes(body))


def layer_table(names):
    body = bytearray(13 + 29 * len(names))
    body[16 - 5] = len(names) & 255
    body[17 - 5] = len(names) >> 8
    for i, name in enumerate(names):
        data = legacy(name)[:24]
        at = 18 + 29 * i - 5
        body[at + 4] = len(data)
        body[at + 5:at + 5 + len(data)] = data
    return block(6, bytes(body))


def container(kind, inner):
    return block(kind, b'\x00' * 3 + b''.join(inner) + b'\x00' * 3)


def tab_label(ref, label, xy):
    rec = bytearray(29 + len(label) + 90)
    rec[0] = len(ref)
    rec[1:1 + len(ref)] = ref.encode('ascii')
    rec[17:21] = f32(1.5)
    rec[25:27] = struct.pack('<H', 7)
    rec[28] = len(label)
    rec[29:29 + len(label)] = label.encode('ascii')
    sep = 29 + len(label)
    rec[sep] = 1
    rec[sep + 1:sep + 5] = u32(42)
    rec[sep + 8:sep + 24] = f64(xy[0]) + f64(xy[1])
    rec[sep + 46:sep + 50] = f32(0.25)
    return bytes(rec)


def tab_segment(ref, points):
    rec = bytearray(120)
    rec[0] = len(ref)
    rec[1:1 + len(ref)] = ref.encode('ascii')
    for at, (x, y) in zip((17, 45, 87, 103), points):
        rec[at:at + 16] = f64(x) + f64(y)
    return bytes(rec)


def tab_words(ref, words):
    rec = bytearray(bytes([len(ref)]) + ref.encode('ascii') + bytes(20))
    for w in words:
        rec += bytes([len(w)]) + w.encode('ascii')
    return bytes(rec)


# ---------------------------------------------------------------- the seeds ----

N0, E0 = 4448000.0, 421000.0  # a Suşehri-like TM39 origin: northing, easting


def seed_everything():
    """Every block type the reader knows, once, on named and coloured layers."""
    s = version('5.2.0.1035N')
    s += mproj(3, 1, 39) + tiled_xml('SRS="5257"')
    s += layer_table(['0', 'PARSEL', 'YAZI', 'İMAR_ŞERİT', '   ', 'PAFTA'])
    s += lex_colours([(0, 0, 0), (255, 0, 0), (0, 0, 1), (0, 128, 0), (10, 20, 30), (40, 50, 60)])
    s += point(N0, E0, '1284', z=1088.5, layer=1)
    s += point(N0 + 10, E0 + 10, 'P2', layer=1, extended=True)
    s += line((N0, E0), (N0 + 20.0, E0 + 20.0), layer=1)
    s += circle((N0 + 50, E0 + 50), 12.5, layer=2, color=1)
    s += arc((N0 + 80, E0 + 80), 20.0, 0.25, 1.5, layer=2, color=255)
    s += arc((N0 + 90, E0 + 90), 5.0, 30.0, 120.0, layer=2, extended=True)
    s += text((N0 + 5, E0 + 5), 'ADA 101 PARSEL ş', 2.5, rotation_rad=math.pi / 6, layer=2)
    s += text((N0 + 6, E0 + 6), 'İKİNCİ', 3.0, layer=2, extended=True)
    s += symbol((N0 + 7, E0 + 7), 12, 1.0, layer=3)
    s += multiline([(N0, E0), (N0, E0 + 30), (N0 + 20, E0 + 30), (N0 + 20, E0), (N0, E0)],
                   label='175', layer=3)
    s += multiline([(N0 + 100, E0), (N0 + 110, E0 + 5), (N0 + 120, E0 + 3)], label='açık', layer=3)
    s += compressed((N0 + 200, E0 + 200), [(0, 0), (1.5, 2.5), (3.0, 4.0), (3.0, 4.0), (6.0, 1.0)],
                    layer=3)
    s += box((N0 + 300, E0 + 300), (N0 + 310, E0 + 325), 0.5, name='plan12', layer=4)
    s += map_sheet((N0 + 400, E0 + 400), (N0 + 1100, E0 + 940), 'H40-D-07-B-1-C', layer=6)
    s += triangle((N0 + 500, E0 + 500), (N0 + 510, E0 + 500), (N0 + 505, E0 + 509), layer=4)
    s += block_ref((N0 + 600, E0 + 600), 'AĞAÇ', rotation_rad=1.0, layer=4)
    s += unknown(8) + unknown(14)
    s += container(0, [line((N0 + 700, E0 + 700), (N0 + 701, E0 + 702), layer=1),
                       point(N0 + 702, E0 + 702, 'İÇ', layer=1)])
    return s


def seed_smart():
    """A SmartObject and the `S0` marks on layer 0 the reader then leaves out."""
    s = layer_table(['0', 'AKILLI'])
    s += smart((N0, E0), 100.0, 60.0, 50.0, layer=1)
    s += smart((N0 + 500, E0), 80.0, 40.0, 0.0, token='GRID_A7', layer=1)
    s += symbol((N0 + 1, E0 + 1), 0, 1.0, layer=0)
    s += symbol((N0 + 2, E0 + 2), 0, 1.0, layer=1)
    s += symbol((N0 + 3, E0 + 3), 5, 1.0, layer=0)
    return s


def seed_late_tables():
    """Geometry before the tables that name its layers: the reference finalises
    the names at the end, and so must a reader that streams."""
    s = line((N0, E0), (N0 + 1, E0 + 1), layer=2)
    s += text((N0 + 2, E0 + 2), 'ÖNCE', 2.0, layer=7)
    s += layer_table(['0', 'BİR', 'İKİ'])
    s += line((N0 + 3, E0 + 3), (N0 + 4, E0 + 4), layer=2)
    s += lex_colours([(1, 2, 3), (4, 5, 6), (7, 8, 9)])
    return s


def seed_broken():
    """Records the reference drops in silence: too short, outside the world, a
    text with no height, a triangle with no area, a sheet with no size, a
    polyline of one point, a compressed curve that goes bad."""
    s = layer_table(['0', 'BOZUK'])
    s += Geometry(1, 40, layer=1).bytes()  # a point too short for its type
    s += point(float('nan'), E0, 'NAN', layer=1)
    s += point(1e9, E0, 'UZAK', layer=1)
    g = Geometry(5, 110, layer=1)
    g.xy(N0, E0).put(97, bytes([3])).put(98, b'YOK')  # no height anywhere
    s += g.bytes()
    s += triangle((N0, E0), (N0 + 1, E0 + 1), (N0 + 2, E0 + 2), layer=1)
    s += map_sheet((N0, E0), (N0, E0 + 5), 'SIFIR', layer=1)
    s += multiline([(N0, E0)], layer=1)
    s += compressed((N0, E0), [(float('nan'), 0), (1, 1), (float('inf'), 0), (float('nan'), 0),
                               (float('nan'), 0), (float('nan'), 0), (2, 2)], layer=1)
    s += box((N0, E0), (N0 + 5, E0 + 5), float('nan'), layer=1)
    s += b'\x15\xff\xff\xff\x7f garbage past the end'
    return s


def seed_attributes():
    """`@TAB` records of the three shapes the reference decodes."""
    s = layer_table(['0'])
    s += point(N0, E0, 'A', layer=0)
    s += tab_label('@TAB1', 'ADA_NO', (N0 + 1.25, E0 + 2.5))
    s += tab_segment('@TAB1', [(N0, E0), (N0 + 1, E0 + 1), (N0 + 2, E0 + 2), (N0 + 3, E0 + 3)])
    s += tab_words('@TAB23', ['MAHALLE', 'KENT', 'MAHALLE', '@TAB23'])
    return s


def seed_planet():
    """Every Planet symbol Netcad 8 writes, and the two records a reader must not
    trust: a class it does not know and a property block that does not add up."""
    s = version('8.5.6.1095')
    s += layer_table(['0', 'SM_YERLESIM', 'SM_YAPILASMA', 'SM_YOL', 'SM_NOT', 'SM_FONKADI'])
    s += lex_colours([(0, 0, 0), (200, 0, 0), (0, 0, 200), (0, 120, 0), (80, 80, 80), (120, 0, 120)])
    n, e = N0 + 1000, E0 + 1000
    s += planet('yerlesim', (n, e), [prop('nizam', 'AYRIK', 'Nizam', kind=0x12), prop('kat', '3', 'Kat', kind=0x09),
                                     prop('chkKatIsNull', 'False', 'Kat', False, 0x03), prop('txtOn', '5', 'Ön'),
                                     prop('chkOnIsNull', 'False', 'Ön', False, 0x03), prop('txtArka', '', 'Arka', kind=0x12),
                                     prop('chkArkaIsNull', 'True', 'Arka', False, 0x03), prop('txtYan', '3', 'Yan'),
                                     prop('chkYanIsNull', 'False', 'Yan', False, 0x03)], layer=1)
    s += planet('yerlesim', (n, e + 30), [prop('nizam', 'BLOK', 'Nizam', kind=0x12), prop('kat', '4', 'Kat', kind=0x09),
                                          prop('chkKatIsNull', 'False', 'Kat', False, 0x03), prop('txtOn', '0', 'Ön'),
                                          prop('chkOnIsNull', 'True', 'Ön', False, 0x03)], layer=1)
    s += planet('yerlesim', (n, e + 60), [prop('nizam', 'BİTİŞİK', 'Nizam', kind=0x12), prop('kat', '2', 'Kat', kind=0x09),
                                          prop('txtOn', '5', 'Ön'), prop('txtArka', '4', 'Arka', kind=0x12),
                                          prop('txtYan', '3', 'Yan')], layer=1)
    s += planet('yapilasma', (n - 30, e), [prop('choiceType', '1', 'ChoiceType', False, 0x09),
                                           prop('numberOfDecimalPlace', '2', 'Ondalık Basamak Sayısı', False, 0x09),
                                           prop('taks', '0.3', 'Taks'), prop('chkTaksIsNull', 'False', 'Taks', False, 0x03),
                                           prop('kaks', '0.9', 'Kaks'), prop('chkKaksIsNull', 'False', 'Kaks', False, 0x03),
                                           prop('minTaks', '0', 'MinTaks'), prop('chkMinTaksIsNull', 'True', 'MinTaks', False, 0x03)],
                  layer=2)
    s += planet('yapilasma', (n - 30, e + 30), [prop('choiceType', '1', 'ChoiceType', False, 0x09),
                                                prop('taks', '0.40', 'Taks'), prop('minTaks', '0.30', 'MinTaks'),
                                                prop('kaks', '0.90', 'Kaks'), prop('minKaks', '0.65', 'MinKaks')], layer=2)
    s += planet('yapilasma', (n - 30, e + 60), [prop('choiceType', '0', 'ChoiceType', False, 0x09),
                                                prop('emsal', '1.5', 'Emsal'), prop('yEncok', '12.5', 'YEnçok'),
                                                prop('chkYEnCokIsNull', 'False', 'YEnçok', False, 0x03),
                                                prop('hmax', '0', 'Hmax'), prop('chkHmaxIsNull', 'True', 'Hmax', False, 0x03)],
                  layer=2)
    s += planet('yol', (n - 60, e), [prop('genislik', '17', 'Genişlik'), prop('yolTuru', '0', 'Yol Türü', kind=0x08)],
                size=1.275, layer=3)
    s += planet('yol', (n - 60, e + 30), [prop('genislik', '7.5', 'Genişlik')], size=0.5625, grads=50.0, layer=3)
    s += planet('fonksiyon', (n - 90, e), [prop('adi', 'TEKNOLOJİ GELİŞTİRME BÖLGESİ', 'Fonksiyon Adı', kind=0x12)],
                layer=5)
    s += planet('plannotu', (n - 100, e + 40), [prop('width', '60', 'width', False, 0x0d), prop('height', '20', 'height', False, 0x0d),
                                                prop('rtfData', rtf_note(), 'rtfData', False, 0x12)], size=0.5, layer=4)
    s += planet('bilinmeyen', (n - 120, e), [prop('renk', 'mavi', 'Renk', kind=0x12)], layer=4)
    s += planet('yol', (n - 120, e + 30), [prop('genislik', '9', 'Genişlik')], layer=3, broken=True)
    return s


def seed_truncated():
    """The first seed cut in the middle of a block."""
    whole = seed_everything()
    return whole[:len(whole) * 2 // 3]


SEEDS = {
    '01-her-tur.ncz': seed_everything,
    '02-akilli-nesne.ncz': seed_smart,
    '03-gec-tablolar.ncz': seed_late_tables,
    '04-bozuk-kayitlar.ncz': seed_broken,
    '05-oznitelik-tablolari.ncz': seed_attributes,
    '06-kesik.ncz': seed_truncated,
    '07-akilli-nesneler.ncz': seed_planet,
}


def main():
    root = pathlib.Path(__file__).resolve().parents[2]
    out = root / 'fixtures/formats/v1/ncz'
    check = '--check' in sys.argv[1:]
    out.mkdir(parents=True, exist_ok=True)
    stale = []
    for name, make in SEEDS.items():
        data = make()
        path = out / name
        if check:
            if not path.exists() or path.read_bytes() != data:
                stale.append(name)
        else:
            path.write_bytes(data)
    if stale:
        print('ncz_reference: güncel değil: ' + ', '.join(stale) + ' (python3 scripts/fixtures/ncz_reference.py)')
        sys.exit(1)
    print(f'ncz_reference: {len(SEEDS)} dosya ' + ('güncel' if check else f'yazıldı ({out})'))


if __name__ == '__main__':
    main()
