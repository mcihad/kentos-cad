"""Otel ve yurt yatak odasının kapısından kaçış uzaklığı: Binaların Yangından Korunması Hakkında Yönetmelik 50/4, 50/5
ve Ek-5/B (docs/adr/8156; plan/bim-plan.md B22, BIM-13'ün konut dışı kullanımları).

- **50/4 (19.12.2007):** kaçış uzaklığı yatak odasının ya da süit odanın çıkış kapısından başlayarak kaçış merdivenine,
  dış kaçış geçidine ya da dış açık alana açılan çıkış kapısına kadar ölçülür. Olguların ``doorExit``'i: odanın
  kapılarından en yakınının dış yanından en yakın kat çıkışına (kaçış merdiveninin mahaline açılan kapı, zemin katta
  dışarı açılan kapı, ortak mahaldeki açık merdiven) yürüyüş. Kaçış merdiveninin mahaline ya da zemin katta dışarı
  açılan kapı çıkıştadır (0 m).
- **50/5:** koridor boyunca yalnız tek yönde kaçış varsa uzaklık en uzaktaki yatak odasının kapısından, iki yönde kaçış
  varsa her yatak odasının kapısından ölçülür. Her oda kendi kapısından ölçülür: tek çıkışa ulaşan odanın bütün yolu tek
  yön sınırına; iki çıkışa ulaşanın tek yönlü kısmı (öbür çıkışa giden yolla ortak kısmı) tek yön, bütünü iki yön
  sınırına. Tek yönlü koridorda en uzaktaki kapının yolu en uzunudur: her kapının sınandığı yerde o da sınanır.
- **Sınırlar (Ek-5/B'nin 9.7.2015 hâli):** Oteller, Pansiyonlar tek yön 15 m (yağmurlamalı 20 m), iki yön 30 m (45 m);
  Yurtlar, Yatakhaneler tek yön 15 m (30 m), iki yön 45 m (75 m). Yağmurlama projenin beyanıdır (``sprinklers``).
- **Kapsam:** konaklama binası (``konaklama.sinif``: kullanımında ya da adında otel, motel, pansiyon, konaklama ya da
  hostel geçen binada otel satırı; yalnız yurt ya da yatakhane geçende yurt satırı). Yatak odası adı Yatak, Süit, Konuk,
  Misafir, Öğrenci odası, Oda ya da Otel odası; banyo, WC, koridor ve hol değil (``yatak_odasi``). Kapısı öbür bir yatak
  odasına açılan oda (süitin iç odası) o odanın kapısından ölçülür: süitin çıkış kapısı (50/4).
- **Ölçülemeyen:** kapısı çizilmemiş ya da kapısından kat çıkışına yol bulunamayan oda eksik veri; çıkışa ulaşamayan
  mahal Mimari denetimin 33/6-b bulgusudur.
- **Masaüstünün yangın kuralları** bu odaları en uzak noktalarından ölçmez (32/4); Kaçış yolları çizimi yolu kapıdan
  çizer (``kentos_domain::fire_use::lodging_of``, ``lodging_bedroom`` aynı okumadır).

**Okunan olgular** (``facts.building``): binaların kullanımı, mahaller (adı, kapısından kat çıkışına yürüyüş: uzunluğu,
tek yönlü kısmı, ulaştığı çıkış sayısı, kapının açıldığı mahal), kapılar (iki yanı), yağmurlama beyanı.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import yururluk as y
from .katman import PAY
from .konaklama import sinif
from .kullanim import _kapilari, _otel_odasi, _sozcukler, _var

MADDELER = [("BYKHY", "50/4")]

#: Geçiş ve ıslak hacimler yatak odası değildir (Süit banyosu, Kat holü).
_ODA_DEGIL = ("KORIDOR", "HOL", "WC", "TUVALET", "BANYO", "DUS", "LAVABO", "ANTRE")


def _m(x: float) -> str:
    """Bir uzaklık, santimetreye; milimetresi varsa milimetreye."""
    return (f"{x:.2f}" if abs(x * 100 - round(x * 100)) < 1e-6 else f"{x:.3f}").replace(".", ",")


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def yatak_odasi(ad: str | None) -> bool:
    """Konaklama binasının yatak odası mı: otel odası, yatak odası, süit, konuk ya da misafir odası, Oda, öğrenci odası;
    banyo, WC, koridor ve hol değil."""
    w = _sozcukler(ad)
    oda = _otel_odasi(ad) or (_var(w, ("OGRENCI",)) and _var(w, ("ODA",)))
    return oda and not _var(w, _ODA_DEGIL)


def _bulgu(ileti: str, onarim: str, hal: y.Hal, uid: str, iz: str, state: str | None = None) -> dict[str, Any]:
    out: dict[str, Any] = {"code": "bykhy_bedroom_escape_distance", "message": f"{ileti} — {hal.kisa()}",
                           "repair": onarim, "uids": [uid], "rule": hal.kural(), "_iz": iz}
    if state:
        out["state"] = state
    return out


def uzakliklar(bina: dict[str, Any], hal: y.Hal, satir: str) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """50/4, 50/5: her yatak odasının kapısından kat çıkışına uzaklık, binanın Ek-5/B satırıyla (``satir``: ``otel``
    ya da ``yurt``)."""
    yagmurlama = bool(bina.get("sprinklers"))
    sutun = 1 if yagmurlama else 0
    tek, iki = (float(hal.anahtar[satir][0][sutun]), float(hal.anahtar[satir][1][sutun]))
    ek = ", yağmurlamalı" if yagmurlama else ""
    liste = [s for s in bina.get("spaces") or [] if yatak_odasi(s.get("name"))]
    if not liste:
        return [], _satir(hal, y.KAPSAM_DISI, "yatak odası yok")
    odalar = {s["uid"] for s in liste}
    bulgular: list[dict[str, Any]] = []
    kalan = eksik = icerde = 0
    en_uzun = 0.0
    for s in liste:
        ad = s.get("name") or "Oda"
        yol = s.get("doorExit")
        if yol is not None and yol.get("into") in odalar:
            # Süitin iç odası: süitin çıkış kapısından, kapısının açıldığı odayla ölçülür.
            icerde += 1
            continue
        if yol is None:
            eksik += 1
            if not _kapilari(bina, s["uid"]):
                ileti = f"“{ad}”: kapısı çizilmemiş; kapısından kat çıkışına uzaklık ölçülemedi"
                onarim = "Odanın kapısını duvarına yerleştirin."
            else:
                ileti = f"“{ad}”: kapısından bir kat çıkışına (kaçış merdiveni, zemin katta dışarı açılan kapı) yol yok"
                onarim = "Katın kaçış merdivenini çizin ya da odanın kapısını ortak koridora açın."
            bulgular.append(_bulgu(ileti, onarim, hal, s["uid"], f"{s['uid']}:yol", y.EKSIK_VERI))
            continue
        uzun, tek_yonlu, cikis = float(yol["length"]), float(yol.get("oneWay") or 0.0), int(yol.get("exits") or 0)
        en_uzun = max(en_uzun, uzun)
        if cikis <= 1:
            if uzun <= tek + PAY:
                continue
            ileti = f"“{ad}” kapısından kat çıkışına {_m(uzun)} m > {_m(tek)} m ({satir}, tek yön{ek})"
            onarim = "Koridora ikinci bir kaçış yönü (merdiven ya da çıkış) açın ya da odayı çıkışa yaklaştırın."
            iz = f"{s['uid']}:tek:{tek}"
        elif tek_yonlu > tek + PAY:
            ileti = (f"“{ad}” kapısından kat çıkışına yolun tek yönlü kısmı {_m(tek_yonlu)} m > {_m(tek)} m "
                     f"({satir}{ek})")
            onarim = "İki çıkışa giden yollar daha erken ayrılmalı: koridoru ya da çıkışları yeniden düzenleyin."
            iz = f"{s['uid']}:tek-yonlu:{tek}"
        elif uzun > iki + PAY:
            ileti = f"“{ad}” kapısından kat çıkışına {_m(uzun)} m > {_m(iki)} m ({satir}, iki yön{ek})"
            onarim = "Çıkışı yaklaştırın ya da yeni bir kaçış merdiveni ekleyin."
            iz = f"{s['uid']}:iki:{iki}"
        else:
            continue
        kalan += 1
        bulgular.append(_bulgu(ileti, onarim, hal, s["uid"], iz))
    olculen = len(liste) - icerde
    if kalan:
        durum = _satir(hal, y.KALDI, f"{kalan} yatak odasının kapısından kat çıkışına uzaklık Ek-5/B'nin {satir} "
                                     "satırını aşıyor")
    elif eksik:
        durum = _satir(hal, y.EKSIK_VERI, f"{eksik} yatak odasının kapısından kat çıkışına yol ölçülemedi")
    else:
        durum = _satir(hal, y.GECTI, f"{olculen} yatak odasının kapısından kat çıkışına en çok {_m(en_uzun)} m "
                                     f"(tek yön {_m(tek)} m, iki yön {_m(iki)} m{ek})")
    return bulgular, durum


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin yatak odası kaçış uzaklığı bulguları."""
    hal = y.hal("BYKHY", "50/4", gun)
    if hal is None:
        ilk = y.haller("BYKHY", "50/4")[0]
        not_ = f"başvuru günü ({y.yaz(gun)}) kural setinin bildiği ilk hâlden ({ilk.etiket}) önce"
        return y.Sonuc([], [{"rule": "BYKHY 50/4", "title": ilk.baslik, "version": None, "gazette": None,
                             "state": y.UZMAN, "note": not_}])
    satir, nereden = sinif(bina)
    if satir is None:
        not_ = f"{nereden} otel, motel, pansiyon, yurt ya da yatakhane değil"
        return y.Sonuc([], [_satir(hal, y.KAPSAM_DISI, not_)])
    bulgular, durum = uzakliklar(bina, hal, satir)
    return y.Sonuc(bulgular, [durum])


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin yatak odası kaçış uzaklığı bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "yatak odası kaçış kuralı")
