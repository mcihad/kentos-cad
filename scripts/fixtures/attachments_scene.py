#!/usr/bin/env python3
"""Nesne eklerinin resimlerinin ve izinin çizimi (docs/adr/0225): bir yapı adasının dört parseli (Ada, Parsel, Mahalle, Nitelik,
Malik), yapıları ve yolları, sahada çekilmiş üç fotoğrafın noktaları (Ad, Tarih, Yön, Kamera; her biri kendi fotoğrafını ek olarak
taşır). 101/5 parselinin ekleri cephe ve bahçe fotoğrafları (gömülü; bahçeninki 101/6'yla aynı varlık), tapu belgesi (bağlı
dosya, notlu) ve imar durumu (adres); 101/6'nınki bahçe fotoğrafı. Parsel katmanının bilgi kartı (başlık, alanlar, metin) ve iki eylemi (haritada aç, tapu belgesi); Fotoğraflar
katmanının kartı. Fotoğraflar projenin kitaplığında, Ekler'de PNG; bağlı tapu belgesi çizimin yanında küçük bir PDF.

    python3 scripts/fixtures/attachments_scene.py          # fixtures/interaction/v1/attachments.kcad'i ve belgeyi yazar
    python3 scripts/fixtures/attachments_scene.py --check  # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz: fotoğraflar küçük bir tarama çizicisiyle, PNG'ler zlib'le (süzgeç Up, düzey 9) yazılır; kimlikleri
içeriklerinin SHA-256'sının ilk 16 onaltılık hanesiyle `resim-…` (ADR 0192 §2).
"""

import base64
import hashlib
import json
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "interaction" / "v1" / "attachments.kcad"
DEED = ROOT / "fixtures" / "interaction" / "v1" / "belgeler" / "tapu-101-5.pdf"
# TUREF / TM33: Ankara, Kızılay'ın güneyi.
E, N = 487000, 4420000
W, H = 320, 240


# --- A small rasteriser -------------------------------------------------------------------------------------------------

def canvas(top, bottom):
    """A sky from `top` to `bottom`, row by row."""
    rows = []
    for y in range(H):
        t = y / (H - 1)
        rows.append([tuple(round(a + (b - a) * t) for a, b in zip(top, bottom))] * W)
    return [list(r) for r in rows]


def rect(img, x0, y0, x1, y1, color):
    for y in range(max(0, y0), min(H, y1)):
        row = img[y]
        for x in range(max(0, x0), min(W, x1)):
            row[x] = color


def circle(img, cx, cy, r, color):
    for y in range(max(0, cy - r), min(H, cy + r + 1)):
        for x in range(max(0, cx - r), min(W, cx + r + 1)):
            if (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
                img[y][x] = color


def triangle(img, a, b, c, color):
    """Pixels whose centres fall inside a, b, c (either winding)."""
    def side(p, q, x, y):
        return (q[0] - p[0]) * (y - p[1]) - (q[1] - p[1]) * (x - p[0])
    xs, ys = [a[0], b[0], c[0]], [a[1], b[1], c[1]]
    for y in range(max(0, min(ys)), min(H, max(ys) + 1)):
        for x in range(max(0, min(xs)), min(W, max(xs) + 1)):
            px, py = x + 0.5, y + 0.5
            s1, s2, s3 = side(a, b, px, py), side(b, c, px, py), side(c, a, px, py)
            if (s1 >= 0 and s2 >= 0 and s3 >= 0) or (s1 <= 0 and s2 <= 0 and s3 <= 0):
                img[y][x] = color


def windows(img, x0, y0, cols, rows, w, h, gap, color):
    for j in range(rows):
        for i in range(cols):
            x, y = x0 + i * (w + gap), y0 + j * (h + gap)
            rect(img, x, y, x + w, y + h, color)


def tree(img, x, y, r):
    rect(img, x - 3, y, x + 3, y + 2 * r, (92, 64, 40))
    circle(img, x, y - r // 3, r, (64, 120, 58))
    circle(img, x - r // 2, y - r // 2, r // 2, (78, 138, 66))


def png(img):
    """The picture as a PNG: 8-bit RGB, every row filtered Up, zlib level 9."""
    raw = bytearray()
    prev = bytes(W * 3)
    for row in img:
        line = bytes(c for px in row for c in px)
        raw.append(2)
        raw.extend((line[i] - prev[i]) & 0xFF for i in range(len(line)))
        prev = line

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    head = struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", head) + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b"")


# --- The photos ---------------------------------------------------------------------------------------------------------

def facade():
    """101/5: a two-storey house with a red roof, a tree and the sun."""
    img = canvas((120, 170, 225), (205, 225, 240))
    rect(img, 0, 178, W, H, (110, 150, 80))
    rect(img, 0, 196, W, 206, (150, 150, 150))
    circle(img, 268, 42, 18, (250, 220, 120))
    rect(img, 70, 92, 230, 190, (226, 206, 168))
    triangle(img, (58, 94), (242, 94), (150, 44), (176, 72, 56))
    windows(img, 86, 104, 4, 2, 22, 24, 16, (92, 128, 160))
    rect(img, 140, 152, 162, 190, (120, 82, 50))
    tree(img, 34, 150, 22)
    return img


def garden():
    """101/6: the garden, a fence and fruit trees."""
    img = canvas((135, 180, 230), (210, 230, 245))
    rect(img, 0, 150, W, H, (98, 146, 72))
    for x in range(0, W, 18):
        rect(img, x, 138, x + 6, 168, (240, 238, 228))
    rect(img, 0, 146, W, 150, (240, 238, 228))
    rect(img, 0, 158, W, 162, (240, 238, 228))
    tree(img, 70, 120, 26)
    tree(img, 168, 112, 30)
    tree(img, 262, 124, 24)
    for (x, y) in [(60, 112), (78, 104), (160, 98), (182, 110), (254, 116)]:
        circle(img, x, y, 4, (220, 90, 60))
    return img


def street():
    """IMG_0412: the street to the south, houses on both sides."""
    img = canvas((110, 160, 220), (200, 220, 238))
    rect(img, 0, 150, W, H, (120, 150, 92))
    triangle(img, (130, 150), (190, 150), (300, 240), (88, 88, 92))
    triangle(img, (130, 150), (20, 240), (300, 240), (88, 88, 92))
    triangle(img, (158, 160), (162, 160), (172, 240), (236, 236, 220))
    triangle(img, (158, 160), (148, 240), (172, 240), (236, 236, 220))
    rect(img, 12, 96, 96, 168, (214, 190, 160))
    triangle(img, (6, 98), (102, 98), (54, 66), (160, 70, 52))
    windows(img, 22, 108, 3, 2, 16, 18, 10, (90, 124, 156))
    rect(img, 222, 104, 306, 166, (200, 206, 214))
    triangle(img, (216, 106), (312, 106), (264, 76), (110, 96, 92))
    windows(img, 232, 116, 3, 2, 16, 16, 10, (80, 110, 140))
    return img


def park():
    """IMG_0413: the corner's park, a bench and trees."""
    img = canvas((128, 176, 228), (206, 228, 244))
    rect(img, 0, 140, W, H, (92, 152, 76))
    triangle(img, (150, 140), (170, 140), (230, 240), (206, 190, 150))
    triangle(img, (150, 140), (90, 240), (230, 240), (206, 190, 150))
    tree(img, 48, 96, 34)
    tree(img, 270, 104, 30)
    tree(img, 200, 92, 20)
    rect(img, 30, 186, 110, 194, (130, 90, 60))
    rect(img, 34, 194, 40, 210, (70, 70, 74))
    rect(img, 100, 194, 106, 210, (70, 70, 74))
    rect(img, 30, 172, 110, 178, (130, 90, 60))
    return img


def manhole():
    """IMG_0415: a manhole cover in the asphalt, from above."""
    img = canvas((78, 78, 82), (96, 96, 100))
    for y in range(0, H, 7):
        for x in range((y * 3) % 11, W, 11):
            img[y][x] = (110, 110, 114)
    circle(img, 160, 120, 82, (60, 62, 66))
    circle(img, 160, 120, 74, (118, 112, 100))
    # The cover's grid, inside its rim.
    for y in range(46, 195):
        for x in range(86, 235):
            on = (x - 160) % 15 in (13, 14, 0, 1) or (y - 120) % 15 in (13, 14, 0, 1)
            if on and (x - 160) ** 2 + (y - 120) ** 2 <= 70 * 70:
                img[y][x] = (88, 84, 76)
    circle(img, 160, 120, 12, (118, 112, 100))
    return img


PHOTOS = [
    ("cephe", facade),
    ("bahce", garden),
    ("IMG_0412", street),
    ("IMG_0413", park),
    ("IMG_0415", manhole),
]


def deed_pdf():
    """A one-page PDF with a line of text: the deed the parcel links to."""
    text = b"BT /F1 18 Tf 72 720 Td (Tapu senedi - Ada 101 Parsel 5) Tj ET"
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        b"<< /Length " + str(len(text)).encode() + b" >>\nstream\n" + text + b"\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    out = bytearray(b"%PDF-1.4\n")
    offsets = []
    for i, o in enumerate(objs, 1):
        offsets.append(len(out))
        out += f"{i} 0 obj\n".encode() + o + b"\nendobj\n"
    xref = len(out)
    out += f"xref\n0 {len(objs) + 1}\n0000000000 65535 f \n".encode()
    for off in offsets:
        out += f"{off:010d} 00000 n \n".encode()
    out += f"trailer\n<< /Size {len(objs) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    return bytes(out)


# --- The drawing --------------------------------------------------------------------------------------------------------

def P(x, y):
    return {"x": E + x, "y": N + y}


def style(color, weight=0.25, **more):
    return {"color": color, "lineType": "continuous", "lineWeight": weight, **more}


def layer(i, name, st, **more):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": st,
            "children": [], **more}


def drawing():
    items, ids = [], {}
    for name, draw in PHOTOS:
        data = png(draw())
        pid = "resim-" + hashlib.sha256(data).hexdigest()[:16]
        ids[name] = pid
        items.append({"kind": "asset", "id": pid, "name": name, "path": ["Ekler"], "format": "png",
                      "data": "data:image/png;base64," + base64.b64encode(data).decode(), "width": W, "height": H})
    entities = []

    def add(e, lid, attrs, **more):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs, **more})

    def box(x0, y0, x1, y1):
        return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]

    parcels = [
        ("5", (-24, 0, -4, 16), "Kargir ev ve bahçesi", "Özel"),
        ("6", (-4, 0, 16, 16), "Bahçe", "Özel"),
        ("7", (-24, -16, -4, 0), "Kargir ev", "Hazine"),
        ("8", (-4, -16, 16, 0), "Arsa", "Özel"),
    ]
    attached = {
        "5": [{"name": "Cephe", "asset": ids["cephe"], "note": "Gül Sokak'tan, kuzeyden"},
              {"name": "Bahçe", "asset": ids["bahce"]},
              {"name": "Tapu belgesi", "file": "belgeler/tapu-101-5.pdf", "note": "2019 tarihli tapu senedinin taraması"},
              {"name": "İmar durumu", "url": "https://imar.example.org/ada/101/parsel/5"}],
        "6": [{"name": "Bahçe", "asset": ids["bahce"]}],
    }
    for no, (x0, y0, x1, y1), what, owner in parcels:
        more = {"attachments": attached[no]} if no in attached else {}
        add({"kind": "polygon", "pts": box(x0, y0, x1, y1)}, "parsel",
            {"Ada": "101", "Parsel": no, "Mahalle": "Kızılay", "Nitelik": what, "Malik": owner}, label=f"101/{no}", **more)
    for (x0, y0, x1, y1) in [(-20, 4, -9, 12), (-20, -12, -10, -4), (2, -11, 10, -5)]:
        add({"kind": "polygon", "pts": box(x0, y0, x1, y1)}, "yapi", {})
    add({"kind": "polyline", "pts": [P(-34, 19), P(25, 19)]}, "yol", {"Ad": "Gül Sokak"})
    add({"kind": "polyline", "pts": [P(19, 24), P(19, -22)]}, "yol", {"Ad": "Lale Sokak"})
    shots = [
        ("IMG_0412", (-14, 17.5), "2024-05-17T10:32:05+03:00", "180.00", "Canon EOS R5"),
        ("IMG_0413", (8, 17.5), "2024-05-17T10:35:41+03:00", "200.50", "Canon EOS R5"),
        ("IMG_0415", (19.5, -9), "2024-05-17T10:41:12+03:00", "0.00", "Apple iPhone 15 Pro"),
    ]
    for name, (x, y), taken, look, camera in shots:
        add({"kind": "point", "p": P(x, y)}, "foto", {"Ad": name, "Tarih": taken, "Yön": look, "Kamera": camera},
            label=name, attachments=[{"name": f"{name}.png", "asset": ids[name]}])
    card = {"title": "Parsel {Ada}/{Parsel}", "fields": ["Mahalle", "Nitelik", "Malik"],
            "text": "Hesaplanan alan: {yuvarla($alan, 2)} m²\nÇevre: {yuvarla($çevre, 2)} m"}
    actions = [{"name": "Haritada aç", "target": "https://www.openstreetmap.org/?mlat={$enlem}&mlon={$boylam}#map=19/{$enlem}/{$boylam}"},
               {"name": "Tapu belgesi", "target": "belgeler/tapu-{Ada}-{Parsel}.pdf"}]
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Nesne ekleri",
        "settings": {"srid": 5255, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 500,
                     "workspace": "gis", "autoSelect": True, "drawingFont": "barlow"},
        "origin": {"x": E, "y": N},
        "layers": [layer("foto", "Fotoğraflar", style("#E5484D", point={"symbol": "ring", "size": 6}),
                         card={"title": "{Ad}", "fields": ["Tarih", "Yön", "Kamera"]}),
                   layer("parsel", "Parsel", style("#3E63DD", 0.35, fill="#3E63DD14"), actions=actions, card=card),
                   layer("yapi", "Yapı", style("#868E96", 0.25, fill="#868E9633")),
                   layer("yol", "Yol", style("#ADB5BD", 0.5))],
        "activeLayer": "parsel",
        "entities": entities,
        "styles": {"items": items, "categories": []},
    }


def main():
    text = json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n"
    deed = deed_pdf()
    if "--check" in sys.argv[1:]:
        stale = [p for p, want in [(OUT, text.encode("utf-8")), (DEED, deed)] if not p.exists() or p.read_bytes() != want]
        for p in stale:
            print(f"{p.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
        if stale:
            return 1
        print(f"{OUT.relative_to(ROOT)}, {DEED.relative_to(ROOT)}: güncel.")
        return 0
    OUT.write_text(text, "utf-8")
    DEED.parent.mkdir(parents=True, exist_ok=True)
    DEED.write_bytes(deed)
    print(f"yazıldı: {OUT.relative_to(ROOT)}, {DEED.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
