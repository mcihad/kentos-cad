"""Kat yüksekliği: Planlı Alanlar İmar Yönetmeliği 28/1-c ve Sivas İmar Yönetmeliği 28/1-c (docs/adr/8083).

Kat yükseklikleri, uygulama imar planında daha fazla belirlenmemişse, döşeme üst kotundan döşeme üst kotuna en fazla:

- **1.10.2017** (RG 3.7.2017/30113): konut bölgelerinde zemin ve normal katlarda 3,60 m.
- **12.8.2023** (RG 32277, 2 nci madde ek ibare): aynı; bodrum hariç 3 katı geçmeyen binalarda 4,00 m. Değişiklik yayımı
  günü yürürlüğe girdi, 28/1-c için geçiş hükmü yok: başvuru gününde yürürlükteki hâl okunur.
- **Sivas 28/1-c (10.11.2022):** konut bölgelerinde zemin ve normal katlarda 3,60 m; 4,00 m'si yok.

**Okuma** (olgular ``building.storeys``: kotu, kattan kata yüksekliği, yatak odaları; ``units``, ``dwellings``):

- Bölge türü projede henüz yazılmıyor: kural konut projesinde konut bölgesi sayar ve iletide söyler. Konut projesi bina
  programında konut nitelikli (ya da niteliği yazılmamış) bağımsız bölümü olan projedir; programsız çizimde yatak odası.
- Zemin ve normal katlar ±0,00 ve üstündeki katlardır; bodrum okunmaz, katları sayılmaz. Yükseklik katın kendi
  yüksekliği, yoksa üstündeki katın kotuna kadardır; ikisi de yoksa eksik veri.
- 28/1-d'nin geleneksel kesme taş yığma binası (4,50 m) çizimde okunmaz; planın daha fazlasını vermesi de.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import katman
from . import yururluk as y

ULUSAL = [("PAİY", "28/1-c")]
YEREL = [("Sivas", "28/1-c")]
KONU = "kat yüksekliği kuralı"
ONARIM = "Kat yüksekliğini düşürün (Proje ayarları › Mimari › Katlar) ya da planın kendi notuna bakın."


def konut(bina: dict[str, Any]) -> bool:
    """Proje konutun mu: programında konut nitelikli ya da niteliği yazılmamış bağımsız bölüm; programsızsa yatak
    odası."""
    if int(bina.get("units") or 0) > 0:
        return int(bina.get("dwellings") or 0) > 0
    return any(int(s.get("bedrooms") or 0) > 0 for s in bina.get("storeys") or [])


def ustteki(bina: dict[str, Any]) -> list[dict[str, Any]]:
    """Zemin ve normal katlar: ±0,00 ve üstündekiler."""
    return [s for s in bina.get("storeys") or [] if s["elevation"] >= -1e-6]


def sinir(hal: y.Hal, katlar: int) -> katman.Sinir:
    """Hâlin ``katlar`` katlı binaya koyduğu en çok yükseklik."""
    a = hal.anahtar
    az = a.get("lowStoreys")
    deger = a["lowMost"] if az is not None and katlar <= int(az) else a["most"]
    return katman.Sinir(float(deger), hal)


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _kat(kat: dict[str, Any], katlar: int, sivas: bool, gun: date) -> y.Sonuc:
    """Bir katın ``gun``deki bulgusu ve maddelerin bu kattaki durumu (satırların notu ``kural``'ın)."""
    hu = y.hal("PAİY", "28/1-c", gun)
    hy = y.hal("Sivas", "28/1-c", gun) if sivas else None
    if hu is None:
        return y.Sonuc([], [])
    uids = list(kat.get("rooms") or [])
    h = kat.get("height")
    if h is None:
        bulgu = {"code": "paiy_storey_height_over",
                 "message": f"“{kat['name']}” katının yüksekliği yazılmamış ve üstünde kat yok: kat yüksekliği "
                            f"okunamıyor — {hu.kisa()}",
                 "repair": "Katın yüksekliğini yazın (Proje ayarları › Mimari › Katlar).", "uids": uids,
                 "rule": hu.kural(), "_iz": f"{kat['level']}:yükseklik", "state": y.EKSIK_VERI}
        satirlar = [_satir(h_, y.EKSIK_VERI, "") for h_ in (hu, hy) if h_ is not None]
        return y.Sonuc([bulgu], satirlar)
    ulusal = sinir(hu, katlar)
    yerel = sinir(hy, katlar) if hy is not None else None
    b = katman.bulgu("paiy_storey_height_over", f"“{kat['name']}” kat yüksekliği", float(h), False, ulusal, yerel,
                     ONARIM, uids, kat["level"], f" (konut bölgesi, {katlar} kat)")
    du, dy = katman.durumlar(float(h), False, ulusal, yerel)
    satirlar = [_satir(hu, du, "")] + ([_satir(hy, dy, "")] if hy is not None and dy is not None else [])
    return y.Sonuc([b] if b else [], satirlar)


def _bilinmiyor(gun: date, uids: list[str]) -> y.Sonuc:
    ilk = y.haller("PAİY", "28/1-c")[0]
    ileti = (f"Başvuru günü ({y.yaz(gun)}) PAİY 28/1-c'nin kural setindeki ilk hâlinden ({ilk.etiket}) önce: o günün "
             f"metni KentOS'ta yok")
    bulgu = {"code": "regulation_text_unknown", "message": ileti,
             "repair": "Kat yüksekliklerini maddenin başvuru günündeki metnine göre uzman incelesin.", "uids": uids,
             "rule": "PAİY 28/1-c", "_iz": "28/1-c:bilinmiyor", "state": y.UZMAN}
    satir = {"rule": "PAİY 28/1-c", "title": ilk.baslik, "version": None, "gazette": None, "state": y.UZMAN,
             "note": ileti}
    return y.Sonuc([bulgu], [satir])


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin kat yüksekliği bulguları: her kat başvuru gününün hâlleriyle; gün yoksa her kat her hâlle denenmiş
    (``yururluk.parcalarla``)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    sivas = bina.get("local") == "sivas"
    maddeler = ULUSAL + (YEREL if sivas else [])
    katlar = ustteki(bina)
    n = len(katlar)
    if not konut(bina) or not katlar:
        # Satırların hâlleri: başvuru gününün, gün yoksa en yenisi.
        hallar = [y.hal(yon, m, gun) if gun is not None else y.haller(yon, m)[-1] for yon, m in maddeler]
        neden_ = "konut projesi değil (bina programında konut nitelikli bölüm yok)" if katlar else "±0,00 üstünde kat yok"
        return y.Sonuc([], [_satir(h, y.KAPSAM_DISI, neden_) for h in hallar if h is not None])
    if gun is not None and y.hal("PAİY", "28/1-c", gun) is None:
        return _bilinmiyor(gun, [u for k in katlar for u in k.get("rooms") or []][:20])
    sonuclar = y.parcalarla(katlar, lambda k, g: _kat(k, n, sivas, g), maddeler, gun, neden, KONU,
                            lambda k: list(k.get("rooms") or []))
    bulgular = [b for s in sonuclar for b in s.bulgular]
    satirlar = []
    for r in y.birlesik(sonuclar):
        sayi = r.pop("sayilar")
        kalan, eksik = sayi.get(y.KALDI, 0), sayi.get(y.EKSIK_VERI, 0)
        if r["state"] == y.EKSIK_VERI:
            r["note"] = f"{eksik} katın sonucu başvuru gününe ya da yazılmamış yüksekliğe bağlı"
        elif r["state"] == y.KALDI:
            r["note"] = f"{kalan} kat sınırı aşıyor (konut bölgesi, {n} kat)"
        else:
            r["note"] = f"{n} katın yüksekliği sınırın içinde (konut bölgesi)"
        satirlar.append(r)
    return y.Sonuc(bulgular, satirlar)
