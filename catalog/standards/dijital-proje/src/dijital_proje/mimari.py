"""Mimari denetimin standart kuralları bir arada (docs/adr/8064, 8065, 8067, 8068, 8074, 8124): EK-3 ve EK-4 (``ek34``),
paftanın modelden üretildiği (``turetim``), acil çıkış beyanı ve kaçış yolları (``cikis``), EK-9'un yazılmış dosyadan
cevapları (``form``; dosya yoksa modelde ölçülen soruları, ``kalite``) ve zorunlu alanlar (``zorunlu``). Masaüstünün
Denetim sekmesi bu işlevi tek süreçte çağırır."""

from __future__ import annotations

from typing import Any

from . import cikis, ek34, form, kalite, turetim, zorunlu


def denetle(facts: dict[str, Any], ifc: dict[str, Any] | None = None) -> list[dict[str, Any]]:
    """Kuralların bulguları, sırayla. ``ifc`` verilince (KentOS'un yazdığı dosyanın olguları, docs/adr/8074) EK-9'un
    soruları dosyadan cevaplanır (``form``): mahal örtüşmesi ve zorunlu alanlar da dosyadandır, çizimden ölçülenleri
    (``kalite``, ``zorunlu``) yinelemez."""
    data = ek34.veri()
    out = ek34.denetle(facts, data) + turetim.denetle(facts, data) + cikis.denetle(facts, data)
    if ifc:
        return out + form.denetle(ifc)
    return out + kalite.denetle(facts, data) + zorunlu.denetle(facts, data)
