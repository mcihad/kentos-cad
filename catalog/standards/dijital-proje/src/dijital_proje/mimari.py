"""Mimari denetimin standart kuralları bir arada (docs/adr/8064, 8065, 8067): EK-3 ve EK-4 (``ek34``), paftanın modelden
üretildiği (``turetim``) ve EK-9'un modelde ölçülen soruları (``kalite``). Masaüstünün Denetim sekmesi bu işlevi tek
süreçte çağırır."""

from __future__ import annotations

from typing import Any

from . import ek34, kalite, turetim


def denetle(facts: dict[str, Any]) -> list[dict[str, Any]]:
    """Üç kuralın bulguları, sırayla."""
    data = ek34.veri()
    return ek34.denetle(facts, data) + turetim.denetle(facts, data) + kalite.denetle(facts, data)
