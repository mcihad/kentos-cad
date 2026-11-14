"""Eklenti Vitrini'nin örnek çizimi: Örnek Belediyesi'nde iki ada, dört imar parseli ve beş yapı (yapay veri).

Depo kökünden çalıştırın:

    .run/py/bin/python samples/eklenti-vitrini/vitrin_cizim.py

``samples/eklenti-vitrini/vitrin.kcad`` yazılır (KentOS GIS ile açın). İçinde:

- **Parsel sınırı** (``parsel``): 101/1, 101/2, 101/3 (üst kenarı yay) ve 102/1; ``ada`` ve ``parsel`` öznitelikli.
- **Yapı** (``yapi``): beş yapı; biri 101/1 ile 101/2'ye taşar, biri 102/1'de TAKS'ı tam 0,40 yapar.

Vitrin'in TAKS aracı, ada/parsel işlevi (etiket, kural, süzgeç), Yapıları numarala ve Yapı yerleştir bu katmanlarla
çalışır. Koordinatlar TUREF / TM36 (EPSG:5256).
"""

from __future__ import annotations

from pathlib import Path

import kentos.cad as cad

HERE = Path(__file__).resolve().parent
OUT = HERE / "vitrin.kcad"

# Çizimin köşesi (TM36): parseller bunun 90 × 65 m'lik çevresinde.
X, Y = 500_000.0, 4_400_000.0


def kare(x0: float, y0: float, x1: float, y1: float) -> list[tuple[float, float]]:
    """Köşeden köşeye bir dikdörtgen, çizimin köşesine göre metre."""
    return [(X + x0, Y + y0), (X + x1, Y + y0), (X + x1, Y + y1), (X + x0, Y + y1)]


def main() -> None:
    doc = cad.Document.new("Eklenti vitrini", srid=5256)
    # KentOS'un CBS şablonunun katmanları: "Parsel sınırı" (parsel) ve "Yapı" (yapi).
    for ada, parsel, pts, yay in [
        ("101", "1", kare(0, 0, 30, 30), None),
        ("101", "2", kare(30, 0, 60, 30), None),
        ("101", "3", kare(60, 0, 90, 30), [0, 0, 0.3, 0]),  # üst kenarı dışa doğru yay
        ("102", "1", kare(0, 40, 20, 65), None),
    ]:
        if yay is None:
            cad.polygon.create(doc, layer_id="parsel", pts=pts, attrs={"ada": ada, "parsel": parsel})
        else:
            cad.polygon.create(doc, layer_id="parsel", pts=pts, bulges=yay, attrs={"ada": ada, "parsel": parsel})
    for pts in [
        kare(2, 5, 22, 25),  # 400 m², 101/1'de
        kare(25, 10, 40, 20),  # 150 m²: 50'si 101/1'de, 100'ü 101/2'de
        kare(42, 15, 54, 27),  # 144 m², 101/2'de
        kare(67, 8, 82, 23),  # 225 m², 101/3'te
        kare(5, 42, 15, 62),  # 200 m² / 500 m² = 0,40: 102/1'de tam sınırda
    ]:
        cad.polygon.create(doc, layer_id="yapi", pts=pts)
    saved = doc.save(OUT)
    print(f"{saved.path} ({saved.bytes} bayt)")


if __name__ == "__main__":
    main()
