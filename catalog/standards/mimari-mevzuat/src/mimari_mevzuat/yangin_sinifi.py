"""Yangına dayanım sınıfının okunması (docs/adr/8078 §8, 8081): TS EN 13501-2'nin yazımı (``EI2 60-S200C5``, ``REI 90``):
harfleri (R taşıyıcılık, E bütünlük, I yalıtım, W ışınım, M mekanik etki), dakikası, duman sızdırmazlığı (``S``) ve
kendiliğinden kapanma (``C``). KentOS beyanı müellifin yazdığı gibi taşır; ne okunduğunu bu kural söyler.
"""

from __future__ import annotations

import re
from typing import Any

#: TS EN 13501-2'nin yangına dayanım sınıfı: harfler (R, E, I, W, M, S, C …), dakikası, ardından duman ve kapanma.
_SINIF = re.compile(r"^\s*([REIWM]+[12]?)\s*(\d{1,3})\b(.*)$", re.IGNORECASE)


def okuma(metin: str | None) -> dict[str, Any] | None:
    """Beyan edilen sınıf, harfleri ne olursa olsun (``R 60`` bir kolonun, ``REI 90`` bir duvarın): harfleri, dakikası,
    duman sızdırmaz (``S…``) ve kendiliğinden kapanır (``C…``) olup olmadığı; okunamazsa yok."""
    if not metin:
        return None
    m = _SINIF.match(metin.strip())
    if not m:
        return None
    harf, dakika, kalan = m.group(1).upper(), int(m.group(2)), m.group(3).upper()
    return {
        "harf": harf,
        "dakika": dakika,
        "duman": re.search(r"(^|[\s\-/])S(200|A|M)?\b", kalan) is not None or "-S" in kalan,
        "kapanma": re.search(r"(^|[\s\-/]|\d)C\d?\b", kalan) is not None,
    }


def sinif(metin: str | None) -> dict[str, Any] | None:
    """Bir kapının ya da bir ayrımın beyanı: bütünlüğü (E) olan sınıf (``okuma``); E'siz ya da okunamazsa yok."""
    s = okuma(metin)
    return s if s is not None and "E" in s["harf"] else None


def ayirir(metin: str | None, dakika: int) -> bool | None:
    """Beyan bir kompartımanı ayırır mı (BYKHY 4/tt: duman ve ısı geçirmez, en az ``dakika``): bütünlük (E) ve yalıtım
    (I) en az o kadar; beyan yoksa ya da okunamıyorsa yok."""
    s = sinif(metin)
    if s is None:
        return None
    return "E" in s["harf"] and "I" in s["harf"] and s["dakika"] >= dakika
