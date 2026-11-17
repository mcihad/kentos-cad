"""Mimari denetimin mevzuat kuralları bir arada (docs/adr/8078): masaüstünün Denetim sekmesi ve başsız ``cad.arch.audit``
bu işlevi tek süreçte çağırır. Başvuru günü bir kez okunur; her kural kendi maddelerinin o günkü hâlleriyle denetler.
"""

from __future__ import annotations

from typing import Any

from . import asansor, emsal, kapilar, merdiven_alti
from . import yururluk as y

#: Sırayla çalışan kurallar: her biri ``kontrol(facts, gun, neden) -> yururluk.Sonuc``.
KURALLAR = (asansor, kapilar, emsal, merdiven_alti)


def denetle(facts: dict[str, Any]) -> dict[str, Any]:
    """Bulgular (``code``, ``message``, ``repair``, ``uids``, ``rule``, durumu ``state``: ``missingData`` ya da
    ``expertReview``, yoksa kaldı), maddelerin özet satırları (``rules``: madde, hâlinin günü, sonucu) ve başvuru günü
    (``date``, ``dateNote``)."""
    gun, neden = y.basvuru(facts.get("permit"))
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    for kural in KURALLAR:
        sonuc = kural.kontrol(facts, gun, neden)
        bulgular += sonuc.bulgular
        satirlar += sonuc.satirlar
    if neden and not any(b["code"] == "regulation_date_needed" for b in bulgular):
        bulgular.append(
            {
                "code": "regulation_date_needed",
                "message": f"{neden} Kuralların sonucu başvuru gününe göre değişmiyor.",
                "repair": "Ruhsat takımı penceresinin Künye bölümünde Başvuru tarihi'ni GG.AA.YYYY yazın.",
                "uids": [],
                "rule": None,
                "state": y.EKSIK_VERI,
            }
        )
    secilemedi = any(r["state"] == y.EKSIK_VERI for r in satirlar)
    if gun is not None:
        not_ = f"Başvuru tarihi künyeden: {y.yaz(gun)}"
    elif secilemedi:
        not_ = f"{neden} Kural seti seçilemedi" if neden else "Başvuru tarihi künyede yok: kural seti seçilemedi"
    elif neden:
        not_ = f"{neden} Sonuç güne bağlı değil, en yeni hâller okundu"
    else:
        not_ = "Başvuru tarihi künyede yok: sonuç güne bağlı değil, en yeni hâller okundu"
    return {
        "date": gun.isoformat() if gun else None,
        "dateNote": not_,
        "rules": satirlar,
        "findings": [{k: v for k, v in b.items() if not k.startswith("_")} for b in bulgular],
    }
