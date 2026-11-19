"""Subasman kotu: Planlı Alanlar İmar Yönetmeliği 4/1-yyy (docs/adr/8153).

- **4/1-yyy (1.10.2017):** subasman kotu (zemin kat taban kotu) binaların zemin kat taban döşemesi üst kotudur; imar
  planında aksine hüküm yoksa ±0,00 kotunun altına düşemez ve +1,20 kotunun üstüne çıkamaz. Binanın ±0,00'ı PAİY'nin
  ±0,00'ıdır, binanın kot aldığı nokta: bina programının binası kendi ±0,00'ını söylüyorsa o (katın ``zero``'su,
  docs/adr/8072), yoksa projeninki; katın kotu taban döşemesinin üst kotudur.
- **Zemin kat:** her binanın (olguların katlarının ``building``'i; binası olmayan katlar projenin) adında Zemin geçen
  katı, yoksa asma kat olmayan katlarından kotu ±0,00'a en yakını. Altına düşen ya da üstüne çıkan kaldı; planın aksine
  hükmü olabilir: KentOS planın bu notunu okumaz, uzman onayıyla işaretlenir.

**Okunan olgular** (``facts.building``): katlar (ad, kot, asma kat, binası, mahalleri).
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import katman
from . import yururluk as y

MADDELER = [("PAİY", "4/1-yyy")]


def _katla(metin: str) -> str:
    tablo = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                           "ş": "S", "Ş": "S", "ü": "U", "Ü": "U"})
    return metin.translate(tablo).upper()


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def zeminler(bina: dict[str, Any]) -> list[dict[str, Any]]:
    """Her binanın zemin katı: adında Zemin geçen katı, yoksa asma kat olmayan katlarından kotu binanın ±0,00'ına en
    yakını; binaların ilk görüldüğü sırayla."""
    gruplar: dict[Any, list[dict[str, Any]]] = {}
    for k in bina.get("storeys") or []:
        gruplar.setdefault(k.get("building"), []).append(k)
    out = []
    for katlar in gruplar.values():
        adli = [k for k in katlar if "ZEMIN" in _katla(k.get("name") or "")]
        adaylar = adli or [k for k in katlar if not k.get("mezzanine")]
        if adaylar:
            out.append(min(adaylar, key=lambda k: abs(float(k["elevation"]) - float(k.get("zero") or 0.0))))
    return out


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin subasman bulguları ve 4/1-yyy'nin satırı."""
    hal = y.hal("PAİY", "4/1-yyy", gun)
    if hal is None:
        return y.Sonuc([], [])
    alt, ust = float(hal.anahtar["least"]), float(hal.anahtar["most"])
    zemin = zeminler(bina)
    if not zemin:
        return y.Sonuc([], [_satir(hal, y.KAPSAM_DISI, "zemin kat yok")])
    bulgular: list[dict[str, Any]] = []
    for k in zemin:
        kot = float(k["elevation"]) - float(k.get("zero") or 0.0)
        if alt - 1e-9 <= kot <= ust + 1e-9:
            continue
        ne = (f"±0,00'ın {katman.metre(-kot)} altında" if kot < alt
              else f"+{katman.metre(kot)}, en çok +{katman.metre(ust)}")
        bulgular.append({
            "code": "paiy_plinth_level",
            "message": f"“{k.get('name') or k['level']}” zemin katın taban döşemesi (subasman kotu) {ne}: 4/1-yyy ±0,00 ile "
                       f"+1,20 arasını ister (imar planında aksine hüküm yoksa) — {hal.kisa()}",
            "repair": "Zemin katın kotunu ±0,00 ile +1,20 arasına getirin (Proje ayarları › Mimari › Katlar); plan aksini "
                      "söylüyorsa uzman onayıyla işaretleyin.",
            "uids": list(k.get("rooms") or [])[:20], "rule": hal.kural(), "_iz": f"subasman:{k['level']}"})
    if bulgular:
        return y.Sonuc(bulgular, [_satir(hal, y.KALDI, f"{len(bulgular)} zemin katın subasmanı ±0,00 ile +1,20 dışında")])
    kotlar = ", ".join(f"{float(k['elevation']) - float(k.get('zero') or 0.0):+.2f}".replace("+0.00", "±0.00")
                       for k in zemin)
    return y.Sonuc([], [_satir(hal, y.GECTI, f"zemin kat {kotlar} m")])


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin subasman bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "subasman kuralı")
