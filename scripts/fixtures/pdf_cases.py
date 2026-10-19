#!/usr/bin/env python3
"""The PDF reader's shared cases (docs/adr/0351 §2; plan/cad.md CAD-REF-17, 18).

    python3 scripts/fixtures/pdf_cases.py           # writes the files
    python3 scripts/fixtures/pdf_cases.py --check   # writes nothing; compares

Writes fixtures/pdf/v1/*.pdf and fixtures/pdf/v1/cases.json with Python's standard library (zlib for Flate), no
KentOS code and no PDF library. Each page is drawn here through a small model (`Page`): every drawing call writes its
content-stream operators and, beside them, what ADR 0351 §2 says the reader gives for it, so the expected answer comes
from the model, never from reading the bytes back:

- points are on the page as it shows: through the current matrix (row vectors, `cm` premultiplies), then the crop box's
  lower left corner taken off, then the page's `Rotate` (90: (x, y) → (y, w − x), 180, 270);
- `re` is a closed part of four corners from (x, y) counterclockwise in its own space; `h` closes the part; a stroke's
  width is `w` times the square root of the matrix's area factor; a dash is any non-empty dash array;
- colours are 8-bit: gray, RGB, CMYK (1 − c)(1 − k) …, each component rounded half up;
- a text's place is its first letter's (the text matrix and the rise at the show), its height the length of the text
  space's y axis on the page, its turn the angle of its x axis; a gap below −200 thousandths in TJ reads as a space;
- an optional content group is a layer, named, hidden when the catalogue's default view has it OFF; a form drawn under
  a group or with its own group draws on that layer; pictures (an image XObject, an inline image) and shadings are
  counted, not drawn;
- a page the file does not have says how many it has; an encrypted file and a file that is no PDF are refused.

The files exercise the cross-reference table, a cross-reference stream with its PNG predictor, object streams, an
incremental update, a broken offset the reader rebuilds from, Flate, ASCIIHex and ASCII85 content, standard fonts,
a Type0 font with a ToUnicode map (Turkish letters), and a curve, kept within 0.01 pt of the curve.
"""
import json
import math
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "pdf" / "v1"


def num(v):
    """A number as a content stream writes it."""
    if float(v).is_integer():
        return str(int(v))
    return f"{v:.6f}".rstrip("0").rstrip(".")


def w(v):
    """A number as written and read back: the model draws with what the file holds."""
    return float(num(v))


def mul(a, b):
    """`a` then `b` (row vectors)."""
    return [a[0] * b[0] + a[1] * b[2], a[0] * b[1] + a[1] * b[3], a[2] * b[0] + a[3] * b[2],
            a[2] * b[1] + a[3] * b[3], a[4] * b[0] + a[5] * b[2] + b[4], a[4] * b[1] + a[5] * b[3] + b[5]]


def apply(m, x, y):
    return [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]]


IDENTITY = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]


def c8(v):
    v = min(max(v, 0.0), 1.0) * 255.0
    return int(math.floor(v + 0.5))


def rgb(vals):
    if len(vals) == 1:
        return [c8(vals[0])] * 3
    if len(vals) == 3:
        return [c8(v) for v in vals]
    k = 1.0 - min(max(vals[3], 0.0), 1.0)
    return [c8((1.0 - vals[0]) * k), c8((1.0 - vals[1]) * k), c8((1.0 - vals[2]) * k)]


class Page:
    """A page drawn: its content and what the reader gives for it."""

    def __init__(self, width, height, *, media=None, crop=None, rotate=0):
        self.media = media or [0, 0, width, height]
        self.crop = crop
        self.rotate = rotate
        x0, y0, x1, y1 = crop or self.media
        w, h = x1 - x0, y1 - y0
        turn = {90: [0.0, -1.0, 1.0, 0.0, 0.0, w], 180: [-1.0, 0.0, 0.0, -1.0, w, h],
                270: [0.0, 1.0, -1.0, 0.0, h, 0.0]}.get(rotate % 360, IDENTITY)
        self.base = mul([1.0, 0.0, 0.0, 1.0, -x0, -y0], turn)
        self.size = (h, w) if rotate % 180 == 90 else (w, h)
        self.ops = []
        self.ctm = list(self.base)
        self.stack = []
        self.stroke, self.fill, self.width, self.dashed = [0, 0, 0], [0, 0, 0], 1.0, False
        self.parts, self.cur = [], []
        self.paths, self.texts = [], []
        self.layer_stack = []
        self.layers, self.hidden = [], []
        self.left = {"images": 0, "shadings": 0, "unreadable": 0, "truncated": False}
        self.resources = {}

    # Graphics state.
    def emit(self, text):
        self.ops.append(text)

    def save(self):
        self.emit("q")
        self.stack.append((list(self.ctm), list(self.stroke), list(self.fill), self.width, self.dashed))

    def restore(self):
        self.emit("Q")
        self.ctm, self.stroke, self.fill, self.width, self.dashed = self.stack.pop()

    def cm(self, *m):
        self.emit(" ".join(num(v) for v in m) + " cm")
        self.ctm = mul([w(v) for v in m], self.ctm)

    def line_width(self, w):
        self.emit(f"{num(w)} w")
        self.width = float(w)

    def dash(self, on):
        self.emit("[3 2] 0 d" if on else "[] 0 d")
        self.dashed = on

    def stroke_rgb(self, *v):
        self.emit(" ".join(num(x) for x in v) + {1: " G", 3: " RG", 4: " K"}[len(v)])
        self.stroke = rgb(v)

    def fill_rgb(self, *v):
        self.emit(" ".join(num(x) for x in v) + {1: " g", 3: " rg", 4: " k"}[len(v)])
        self.fill = rgb(v)

    # Paths.
    def move(self, x, y):
        if len(self.cur) > 1:
            self.parts.append((self.cur, False))
        self.emit(f"{num(x)} {num(y)} m")
        p = apply(self.ctm, x, y)
        self.cur, self.start, self.last = [p], p, p

    def line(self, x, y):
        self.emit(f"{num(x)} {num(y)} l")
        p = apply(self.ctm, x, y)
        self.cur.append(p)
        self.last = p

    def curve(self, x1, y1, x2, y2, x3, y3):
        """A cubic: the expected part ends at its end point; the points between are checked on the curve."""
        self.emit(" ".join(num(v) for v in (x1, y1, x2, y2, x3, y3)) + " c")
        ctrl = [self.last, apply(self.ctm, x1, y1), apply(self.ctm, x2, y2), apply(self.ctm, x3, y3)]
        self.cur.append({"curve": ctrl})
        self.last = ctrl[3]

    def close(self):
        self.emit("h")
        if len(self.cur) > 1:
            self.parts.append((self.cur, True))
        self.cur = []
        self.last = self.start

    def rect(self, x, y, w, h):
        if len(self.cur) > 1:
            self.parts.append((self.cur, False))
        self.emit(f"{num(x)} {num(y)} {num(w)} {num(h)} re")
        ring = [apply(self.ctm, x, y), apply(self.ctm, x + w, y), apply(self.ctm, x + w, y + h), apply(self.ctm, x, y + h)]
        self.parts.append((ring, True))
        self.cur = []
        self.start = self.last = ring[0]

    def paint(self, op):
        self.emit(op)
        if len(self.cur) > 1:
            self.parts.append((self.cur, op in ("s", "b", "b*")))
        self.cur = []
        painted, self.parts = self.parts, []
        if op == "n" or not painted:
            return
        det = abs(self.ctm[0] * self.ctm[3] - self.ctm[1] * self.ctm[2])
        stroke = {"color": list(self.stroke), "width": self.width * math.sqrt(det), "dashed": self.dashed} \
            if op in ("S", "s", "B", "B*", "b", "b*") else None
        fill = list(self.fill) if op in ("f", "F", "f*", "B", "B*", "b", "b*") else None
        self.paths.append({"parts": painted, "stroke": stroke, "fill": fill, "layer": self.layer()})

    # Layers.
    def layer(self):
        for l_ in reversed(self.layer_stack):
            if l_ is not None:
                return l_
        return None

    def begin_group(self, prop, name, off=False):
        self.emit(f"/OC /{prop} BDC")
        if name not in self.layers:
            self.layers.append(name)
            self.hidden.append(off)
        self.layer_stack.append(self.layers.index(name))

    def begin_marked(self):
        self.emit("/Span BMC")
        self.layer_stack.append(None)

    def end(self):
        self.emit("EMC")
        self.layer_stack.pop()

    # Texts.
    def text(self, font, size, tm, items, *, rise=0.0, hscale=100.0, words=None, render=0):
        """One text object: `Tf`, `Tm`, `Tz`, `Ts`, `Tr` and a TJ of `items` (strings as their bytes, numbers kerning);
        `words` what it reads as."""
        parts = ["BT", f"/{font} {num(size)} Tf", " ".join(num(v) for v in tm) + " Tm"]
        if hscale != 100.0:
            parts.append(f"{num(hscale)} Tz")
        if rise:
            parts.append(f"{num(rise)} Ts")
        if render:
            parts.append(f"{render} Tr")
        shown = []
        for it in items:
            shown.append(num(it) if isinstance(it, (int, float)) else "<" + it.hex() + ">")
        parts.append("[" + " ".join(shown) + "] TJ")
        parts.append("ET")
        self.emit(" ".join(parts))
        if render in (3, 7) or words is None:
            return
        trm = mul([w(size) * w(hscale) / 100.0, 0.0, 0.0, w(size), 0.0, w(rise)], mul([w(v) for v in tm], self.ctm))
        self.texts.append({"p": [trm[4], trm[5]], "height": math.hypot(trm[2], trm[3]),
                           "angle": math.atan2(trm[1], trm[0]), "text": words, "color": list(self.fill),
                           "layer": self.layer()})

    def form(self, name, inner, matrix=None, own_layer=None):
        """A form XObject drawn here: `inner(page)` draws its content in the form's space."""
        self.emit(f"/{name} Do")
        sub = Page(1, 1)
        sub.ctm = mul(list(map(float, matrix or IDENTITY)), self.ctm)
        sub.stroke, sub.fill, sub.width, sub.dashed = list(self.stroke), list(self.fill), self.width, self.dashed
        sub.layers, sub.hidden = self.layers, self.hidden
        inner(sub)
        layer = own_layer if own_layer is not None else self.layer()
        for p in sub.paths:
            if p["layer"] is None:
                p["layer"] = layer
            self.paths.append(p)
        for t in sub.texts:
            if t["layer"] is None:
                t["layer"] = layer
            self.texts.append(t)
        for k in ("images", "shadings", "unreadable"):
            self.left[k] += sub.left[k]
        self.resources.setdefault("forms", {})[name] = (sub, matrix)

    def image(self, name):
        self.emit(f"/{name} Do")
        self.left["images"] += 1
        self.resources.setdefault("images", []).append(name)

    def inline_image(self):
        self.emit("BI /W 1 /H 1 /BPC 8 /CS /G ID \x80 EI")
        self.left["images"] += 1

    def shading(self):
        self.emit("/Sh1 sh")
        self.left["shadings"] += 1

    def content(self):
        return "\n".join(self.ops).encode("latin-1")

    def expected(self, number, count):
        return {"number": number, "count": count, "width": float(self.size[0]), "height": float(self.size[1]),
                "layers": self.layers, "hidden": self.hidden, "paths": self.paths, "texts": self.texts,
                "left": self.left}


# ── Writing the file ──────────────────────────────────────────────────────

class Writer:
    """Objects numbered from 1, written with a cross-reference table, or (`stream`) with a cross-reference stream
    and the dictionaries packed into an object stream."""

    def __init__(self):
        self.objects = []

    def add(self, body):
        self.objects.append(body)
        return len(self.objects)

    def set(self, n, body):
        self.objects[n - 1] = body

    def table(self, *, header=b"%PDF-1.4\n", broken=False, encrypt=False):
        out = bytearray(header + b"%\xe2\xe3\xcf\xd3\n")
        at = []
        for i, body in enumerate(self.objects):
            at.append(len(out))
            out += f"{i + 1} 0 obj\n".encode() + body + b"\nendobj\n"
        xref = len(out)
        out += f"xref\n0 {len(self.objects) + 1}\n0000000000 65535 f \n".encode()
        for a in at:
            out += f"{a + (7 if broken else 0):010} 00000 n \n".encode()
        extra = b" /Encrypt << /Filter /Standard /V 1 /R 2 /O <00> /U <00> /P -4 >>" if encrypt else b""
        out += f"trailer\n<< /Size {len(self.objects) + 1} /Root 1 0 R".encode() + extra + b" >>\n"
        out += f"startxref\n{xref}\n%%EOF\n".encode()
        return bytes(out)

    def stream(self, packed):
        """`packed`: the object numbers that go in an object stream (dictionaries only)."""
        out = bytearray(b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n")
        n = len(self.objects)
        objstm = n + 1
        xref_n = n + 2
        entries = {}
        head, body = [], bytearray()
        for k, i in enumerate(packed):
            head.append(f"{i} {len(body)}")
            body += self.objects[i - 1] + b"\n"
            entries[i] = (2, objstm, k)
        for i, b in enumerate(self.objects, 1):
            if i in packed:
                continue
            entries[i] = (1, len(out), 0)
            out += f"{i} 0 obj\n".encode() + b + b"\nendobj\n"
        first = " ".join(head).encode() + b"\n"
        data = zlib.compress(first + bytes(body))
        entries[objstm] = (1, len(out), 0)
        out += (f"{objstm} 0 obj\n<< /Type /ObjStm /N {len(packed)} /First {len(first)} /Filter /FlateDecode "
                f"/Length {len(data)} >>\nstream\n").encode() + data + b"\nendstream\nendobj\n"
        entries[xref_n] = (1, len(out), 0)
        rows = [bytes([0, 0, 0, 0, 0])]
        for i in range(1, xref_n + 1):
            t, a, b = entries[i]
            rows.append(bytes([t]) + a.to_bytes(3, "big") + bytes([b]))
        # PNG Up predictor on each row (each byte the difference from the one above).
        prev = bytes(5)
        coded = bytearray()
        for r in rows:
            coded += b"\x02" + bytes((x - y) % 256 for x, y in zip(r, prev))
            prev = r
        data = zlib.compress(bytes(coded))
        out += (f"{xref_n} 0 obj\n<< /Type /XRef /Size {xref_n + 1} /W [1 3 1] /Root 1 0 R /Filter /FlateDecode "
                f"/DecodeParms << /Predictor 12 /Columns 5 >> /Length {len(data)} >>\nstream\n").encode()
        xref_at = entries[xref_n][1]
        out += data + b"\nendstream\nendobj\n"
        out += f"startxref\n{xref_at}\n%%EOF\n".encode()
        return bytes(out)


def stream_obj(data, filters=()):
    """A stream object: `filters` among Flate, AHx, A85, applied in the order listed (the first undone first)."""
    names = []
    for f in reversed(filters):
        if f == "Flate":
            data = zlib.compress(data)
            names.insert(0, "/FlateDecode")
        elif f == "AHx":
            data = data.hex().encode() + b">"
            names.insert(0, "/ASCIIHexDecode")
        elif f == "A85":
            import base64
            data = base64.a85encode(data) + b"~>"
            names.insert(0, "/ASCII85Decode")
    flt = ""
    if names:
        flt = " /Filter " + (names[0] if len(names) == 1 else "[" + " ".join(names) + "]")
    return f"<< /Length {len(data)}{flt} >>\nstream\n".encode() + data + b"\nendstream"


STANDARD = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"


def type0(w):
    """A Type0 font with two-byte codes and a ToUnicode map: Ş ş İ ı Ğ ğ Ü ü Ö ö Ç ç at 1–12, space at 13."""
    letters = "ŞşİıĞğÜüÖöÇç "
    cmap = ["/CIDInit /ProcSet findresource begin 12 dict begin begincmap",
            "/CMapName /KentOS-UCS def 1 begincodespacerange <0000> <FFFF> endcodespacerange",
            f"{len(letters)} beginbfchar"]
    for k, ch in enumerate(letters, 1):
        cmap.append(f"<{k:04X}> <{ord(ch):04X}>")
    cmap.append("endbfchar endcmap CMapName currentdict /CMap defineresource pop end end")
    tu = w.add(stream_obj("\n".join(cmap).encode()))
    desc = w.add(b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /KentosSans /CIDSystemInfo << /Registry (Adobe) "
                 b"/Ordering (Identity) /Supplement 0 >> /DW 600 >>")
    return w.add(f"<< /Type /Font /Subtype /Type0 /BaseFont /KentosSans /Encoding /Identity-H "
                 f"/DescendantFonts [{desc} 0 R] /ToUnicode {tu} 0 R >>".encode()), letters


def tr_codes(letters, text):
    return b"".join((letters.index(ch) + 1).to_bytes(2, "big") for ch in text)


def one_file(pages, *, ocgs=(), off=(), xref="table", filters=(), packed=False, broken=False, encrypt=False,
             fonts=None, extra=None):
    """A file of `pages` (each `(Page, its resources' text)`)."""
    w = Writer()
    cat = w.add(b"")
    tree = w.add(b"")
    # A layer's name as a text string: UTF-16 with its byte order mark.
    ocg_ids = [w.add(f"<< /Type /OCG /Name <FEFF{name.encode('utf-16-be').hex().upper()}> >>".encode()) for name in ocgs]
    kids = []
    for page, resources in pages:
        content = w.add(stream_obj(page.content(), filters))
        attrs = f"/MediaBox [{' '.join(num(v) for v in page.media)}]"
        if page.crop:
            attrs += f" /CropBox [{' '.join(num(v) for v in page.crop)}]"
        if page.rotate:
            attrs += f" /Rotate {page.rotate}"
        res = resources(w, ocg_ids) if resources else "<< >>"
        kids.append(w.add(f"<< /Type /Page /Parent {tree} 0 R {attrs} /Contents {content} 0 R /Resources {res} >>".encode()))
    w.set(tree, f"<< /Type /Pages /Kids [{' '.join(f'{k} 0 R' for k in kids)}] /Count {len(kids)} >>".encode())
    oc = ""
    if ocgs:
        all_ = " ".join(f"{i} 0 R" for i in ocg_ids)
        offs = " ".join(f"{ocg_ids[k]} 0 R" for k in off)
        oc = f" /OCProperties << /OCGs [{all_}] /D << /Order [{all_}] /OFF [{offs}] >> >>"
    w.set(cat, f"<< /Type /Catalog /Pages {tree} 0 R{oc} >>".encode())
    if xref == "stream":
        dicts = [i for i, b in enumerate(w.objects, 1) if b"stream\n" not in b]
        return w.stream(dicts if packed else [])
    return w.table(broken=broken, encrypt=encrypt)


# ── The cases ─────────────────────────────────────────────────────────────

files, expected = {}, {}


def lines_page():
    p = Page(595, 842)
    p.stroke_rgb(1, 0, 0)
    p.line_width(2)
    p.move(10, 10)
    p.line(110, 10)
    p.paint("S")
    p.fill_rgb(0, 0, 1)
    p.rect(20, 20, 50, 30)
    p.paint("f")
    p.save()
    p.cm(2, 0, 0, 2, 100, 100)
    p.stroke_rgb(0.5)
    p.dash(True)
    p.line_width(0.5)
    p.move(0, 0)
    p.line(10, 0)
    p.line(10, 10)
    p.close()
    p.fill_rgb(0, 0.2, 0.4, 0.1)
    p.paint("B")
    p.restore()
    p.move(300, 300)
    p.line(320, 300)
    p.move(300, 310)
    p.line(320, 310)
    p.line(320, 330)
    p.paint("s")
    p.move(0, 0)
    p.line(1, 1)
    p.paint("n")
    p.fill_rgb(0, 0.5, 0)
    p.text("F1", 12, [1, 0, 0, 1, 100, 200], [b"Ada 101"], words="Ada 101")
    a = math.radians(30)
    p.text("F1", 10, [math.cos(a), math.sin(a), -math.sin(a), math.cos(a), 200, 400],
           [b"Yol", -300, b"Kenar"], words="Yol Kenar", rise=2, hscale=80)
    p.text("F1", 8, [1, 0, 0, 1, 50, 50], [b"Gizli"], words=None, render=3)
    return p


def fonts_only(w, _ocgs):
    f = w.add(STANDARD)
    return f"<< /Font << /F1 {f} 0 R >> >>"


def add_file(name, data, pages_expected, errors=None):
    files[name] = data
    expected[name] = {"pages": pages_expected, "errors": errors or {}}


p1 = lines_page()
add_file("cizgiler.pdf", one_file([(p1, fonts_only)]), {"1": p1.expected(1, 1)},
         {"2": {"noPage": 1}, "0": {"noPage": 1}})
p1b = lines_page()
add_file("sikistirilmis.pdf", one_file([(p1b, fonts_only)], filters=("Flate",)), {"1": p1b.expected(1, 1)})
p1c = lines_page()
add_file("ascii.pdf", one_file([(p1c, fonts_only)], filters=("A85", "AHx")), {"1": p1c.expected(1, 1)},
         )
p1d = lines_page()
add_file("xref-akisi.pdf", one_file([(p1d, fonts_only)], xref="stream", packed=True, filters=("Flate",)),
         {"1": p1d.expected(1, 1)})
p1e = lines_page()
add_file("bozuk-xref.pdf", one_file([(p1e, fonts_only)], broken=True), {"1": p1e.expected(1, 1)})


def turned_pages():
    a = Page(400, 300, crop=[50, 20, 350, 220], rotate=90)
    a.stroke_rgb(0, 0, 0)
    a.move(50, 20)
    a.line(350, 20)
    a.line(350, 220)
    a.paint("S")
    b = Page(200, 100, rotate=270)
    b.fill_rgb(1, 1, 0)
    b.rect(10, 10, 30, 20)
    b.paint("f")
    c = Page(200, 100, rotate=180)
    c.move(0, 0)
    c.line(200, 100)
    c.paint("S")
    return a, b, c


ta, tb, tc = turned_pages()
add_file("donuk.pdf", one_file([(ta, None), (tb, None), (tc, None)]),
         {"1": ta.expected(1, 3), "2": tb.expected(2, 3), "3": tc.expected(3, 3)}, {"4": {"noPage": 3}})


def layered():
    p = Page(600, 400)
    p.begin_group("L1", "Yapılar")
    p.stroke_rgb(0, 0, 0.6)
    p.rect(10, 10, 100, 50)
    p.paint("S")
    p.begin_marked()
    p.move(10, 100)
    p.line(110, 100)
    p.paint("S")
    p.end()
    p.end()
    p.begin_group("L2", "Kotlar", off=True)
    p.fill_rgb(0)
    p.text("F1", 9, [1, 0, 0, 1, 20, 300], [b"812.40"], words="812.40")
    p.end()

    def kapak(q):
        q.stroke_rgb(0.6, 0, 0)
        q.line_width(1)
        q.move(0, 0)
        q.line(10, 0)
        q.line(10, 10)
        q.paint("S")
        q.text("F1", 5, [1, 0, 0, 1, 1, 1], [b"R"], words="R")

    p.save()
    p.cm(1, 0, 0, 1, 200, 200)
    p.form("Fm1", kapak, matrix=[2, 0, 0, 2, 5, 5])
    p.restore()
    p.begin_group("L1", "Yapılar")
    p.form("Fm2", kapak, matrix=[1, 0, 0, 1, 400, 50])
    p.end()
    p.form("Fm3", kapak, matrix=[0, 1, -1, 0, 500, 300], own_layer=p.layers.index("Kotlar"))
    p.image("Im1")
    p.inline_image()
    p.shading()
    return p


def layered_resources(w, ocgs):
    f = w.add(STANDARD)
    img = w.add(b"<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /BitsPerComponent 8 /ColorSpace /DeviceGray "
                b"/Length 1 >>\nstream\n\x80\nendstream")
    fm = {}
    for name, (sub, matrix) in lp.resources["forms"].items():
        content = sub.content()
        oc = f" /OC {ocgs[1]} 0 R" if name == "Fm3" else ""
        m = f" /Matrix [{' '.join(num(v) for v in matrix)}]" if matrix else ""
        fm[name] = w.add(f"<< /Type /XObject /Subtype /Form /BBox [0 0 20 20]{m}{oc} /Resources << /Font << /F1 {f} 0 R >> >> "
                         f"/Length {len(content)} >>\nstream\n".encode() + content + b"\nendstream")
    sh = w.add(b"<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 1 0] /Function << /FunctionType 2 /Domain [0 1] "
               b"/C0 [0 0 0] /C1 [1 1 1] /N 1 >> >>")
    xobjects = " ".join(f"/{n} {i} 0 R" for n, i in fm.items())
    return (f"<< /Font << /F1 {f} 0 R >> /XObject << {xobjects} /Im1 {img} 0 R >> /Shading << /Sh1 {sh} 0 R >> "
            f"/Properties << /L1 {ocgs[0]} 0 R /L2 {ocgs[1]} 0 R >> >>")


lp = layered()
add_file("katmanlar.pdf", one_file([(lp, layered_resources)], ocgs=("Yapılar", "Kotlar"), off=(1,)),
         {"1": lp.expected(1, 1)})


def turkish_resources(w, _ocgs):
    f, _letters = type0(w)
    return f"<< /Font << /F2 {f} 0 R >> >>"


# The Type0 font's letters (`type0`): the text drawn with their two-byte codes, read back through its ToUnicode map.
LETTERS = "ŞşİıĞğÜüÖöÇç "
tp = Page(300, 200)
tp.fill_rgb(0.2, 0.2, 0.2)
tp.text("F2", 14, [1, 0, 0, 1, 30, 100], [tr_codes(LETTERS, "Şİığ Çüö")], words="Şİığ Çüö")
add_file("turkce.pdf", one_file([(tp, turkish_resources)]), {"1": tp.expected(1, 1)})


cp = Page(200, 200)
cp.stroke_rgb(0, 0, 0)
cp.move(10, 10)
cp.curve(10, 110, 190, 110, 190, 10)
cp.line(100, 0)
cp.paint("S")
add_file("egri.pdf", one_file([(cp, None)]), {"1": cp.expected(1, 1)})


def incremental():
    """The first version's page draws one line; an update replaces its content with two."""
    p_old = Page(100, 100)
    p_old.move(0, 0)
    p_old.line(50, 0)
    p_old.paint("S")
    data = one_file([(p_old, None)])
    new = Page(100, 100)
    new.move(0, 10)
    new.line(50, 10)
    new.paint("S")
    new.move(0, 20)
    new.line(50, 20)
    new.paint("S")
    content = stream_obj(new.content())
    prev = int(data.rsplit(b"startxref\n", 1)[1].split(b"\n")[0])
    at = len(data)
    data += b"3 0 obj\n" + content + b"\nendobj\n"
    xref = len(data)
    data += f"xref\n3 1\n{at:010} 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R /Prev {prev} >>\nstartxref\n{xref}\n%%EOF\n".encode()
    return data, new


inc, inc_page = incremental()
add_file("guncelleme.pdf", inc, {"1": inc_page.expected(1, 1)})

enc_page = Page(100, 100)
add_file("sifreli.pdf", one_file([(enc_page, None)], encrypt=True), {}, {"1": {"encrypted": True}})
add_file("pdf-degil.pdf", b"Bu bir PDF degil.\n", {}, {"1": {"notPdf": True}})


# ── The answer as the cases hold it ───────────────────────────────────────

def expect_page(e):
    """A page's expectation: each part's points, a curve as `{"curve": [p0, p1, p2, p3]}` standing for the points the
    reader gives for it, each within 0.01 pt of the curve, the last its end."""
    paths = [{**pa, "parts": [{"points": pts, "closed": closed} for pts, closed in pa["parts"]]} for pa in e["paths"]]
    return {**e, "paths": paths}


def build():
    cases = []
    for name in files:
        ex = expected[name]
        cases.append({"file": name, "pages": {k: expect_page(v) for k, v in ex["pages"].items()},
                      "errors": ex["errors"]})
    return json.dumps({"format": "kentos.pdf-cases", "version": 1,
                       "note": "Written by scripts/fixtures/pdf_cases.py from docs/adr/0351 §2; do not edit by hand.",
                       "cases": cases}, ensure_ascii=False, indent=1) + "\n"


def main():
    text = build()
    check = "--check" in sys.argv[1:]
    bad = []
    for name, data in files.items():
        path = OUT / name
        if check:
            if not path.exists() or path.read_bytes() != data:
                bad.append(name)
        else:
            OUT.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    cases_path = OUT / "cases.json"
    if check:
        if not cases_path.exists() or cases_path.read_text(encoding="utf-8") != text:
            bad.append("cases.json")
        if bad:
            sys.exit("güncel değil: " + ", ".join(bad))
        print(f"{len(files)} dosya tutarlı")
        return
    cases_path.write_text(text, encoding="utf-8")
    print(f"{len(files)} dosya yazıldı")


main()
