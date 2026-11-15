"""Mimari denetimin standart kuralları bir arada (docs/adr/8064, 8065): EK-3 ve EK-4 (``ek34``) ile paftanın modelden
üretildiği (``turetim``). Masaüstünün Denetim sekmesi bu işlevi tek süreçte çağırır."""

from __future__ import annotations

from typing import Any

from . import ek34, turetim


def denetle(facts: dict[str, Any]) -> list[dict[str, Any]]:
    """İki kuralın bulguları, sırayla."""
    data = ek34.veri()
    return ek34.denetle(facts, data) + turetim.denetle(facts, data)
