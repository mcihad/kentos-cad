"""Ruhsat yayınının kapısı (docs/adr/8065, 8066): paftaların modelden üretildiği (``turetim``) ve 2B paketin dosyaları
(``paket``). Masaüstünün Paftaları toplu yayınla penceresi, Ruhsat takımından açıldığında, bu işlevi tek süreçte çağırır."""

from __future__ import annotations

from typing import Any

from . import ek34, paket, turetim


def denetle(facts: dict[str, Any]) -> list[dict[str, Any]]:
    """İki kuralın bulguları, sırayla; hepsi hata."""
    data = ek34.veri()
    return turetim.denetle(facts, data) + paket.denetle(facts, data)
