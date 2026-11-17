"""Otel yatak odasının ve süitin içindeki kaçış uzaklığı: Binaların Yangından Korunması Hakkında Yönetmelik 50/3
(docs/adr/8111; plan/bim-plan.md B22, BIM-13'ün konut dışı kullanımları).

- **50/3 (19.12.2007):** otel yatak odasında ya da süit odada en uzak noktadan çıkış kapısına uzaklık 15 m'yi aşmıyorsa
  tek kaçış kapısı yeter. (a) Aşıyorsa birbirinden uzakta en az 2 çıkış kapısı gerekir. (b) Tamamı yağmurlamalı otelde
  en uzak noktadan kapıya uzaklık 20 m'yi aşmaz.
- **Kapsam:** otel, motel ve pansiyon (``kullanim.siniflar``'ın konaklama sınıfı); yalnız yurt ya da yatakhane olan
  binada madde kapsam dışı (50/2 ve 50/3 otel yatak odasını söyler). Oda adı otel odası, yatak odası, süit, konuk ya da
  misafir odası ya da Oda; banyo, WC, koridor ve hol değil.
- **Uzaklık:** olguların ``toDoor``'u: odanın en uzak noktasından (duvarlardan 0,40 m önde, 32/6) en yakın kapısına ya da
  eşiksiz boşluğuna, odanın içinden yürüyüş. Kapı sayısı olguların kapılarından. "Birbirinden uzakta" bir ölçü değildir:
  iki kapılı odada kapıların yeri idarenin takdirindedir.
- **Yağmurlama:** projenin beyanı (``settings.arch.sprinklers``, olgularda ``sprinklers``) bütün otel içindir; sınır 20 m.
  20 m'yi aşan iki kapılı oda uzman incelemesi: (b) aşmamasını ister, (a) iki kapıyı izin verir, ikisinin birlikte okunuşu
  tek anlamlı değil.
- **Süit:** birden çok mahal olarak çizilmiş süitte her mahal kendi en yakın kapısına ölçülür; süitin çıkış kapısına
  yürüyüş ölçülmez (sıradaki iş).

**Okunan olgular** (``facts.building``): binaların kullanımı, mahaller (adı, katı, en uzak noktasından kapısına yürüyüş),
kapılar (iki yanı), yağmurlama beyanı.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import yururluk as y
from .kullanim import _kapilari, _otel_odasi, _sozcukler, _var, siniflar

MADDELER = [("BYKHY", "50/3")]

#: Yurt ve yatakhane otel değildir.
_YURT = ("YURT", "YURD", "YATAKHANE")
#: Geçiş ve ıslak hacimler otel odası değildir (Süit banyosu, Kat holü).
_ODA_DEGIL = ("KORIDOR", "HOL", "WC", "TUVALET", "BANYO", "DUS", "LAVABO", "ANTRE")


def _m(x: float) -> str:
    """Bir uzaklık, santimetreye; milimetresi varsa milimetreye."""
    return (f"{x:.2f}" if abs(x * 100 - round(x * 100)) < 1e-6 else f"{x:.3f}").replace(".", ",")


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _bulgu(ileti: str, onarim: str, hal: y.Hal, uids: list[str], iz: str, bent: str,
           state: str | None = None) -> dict[str, Any]:
    out: dict[str, Any] = {"code": "bykhy_hotel_room_distance", "message": f"{ileti} — {hal.kisa(bent)}",
                           "repair": onarim, "uids": uids, "rule": hal.kural(bent), "_iz": iz}
    if state:
        out["state"] = state
    return out


def otel(bina: dict[str, Any]) -> tuple[bool, str]:
    """Bina 50/3'ün oteli mi: konaklama sınıfında (otel, motel, pansiyon) ve yalnız yurt ya da yatakhane değil; nereden
    okunduğu."""
    sinif_, nereden = siniflar(bina)
    if "konaklama" not in sinif_:
        return False, f"{nereden} otel, motel ya da pansiyon değil"
    kullanim = list(bina.get("uses") or [])
    sozler = [_sozcukler(u) for u in kullanim] if kullanim else [_sozcukler(s.get("name"))
                                                               for s in bina.get("spaces") or []]
    konaklayan = [w for w in sozler if _var(w, ("OTEL", "MOTEL", "PANSIYON", "KONAKLAMA", "HOSTEL", "YURT", "YURD",
                                                   "YATAKHANE"))]
    if konaklayan and all(_var(w, _YURT) for w in konaklayan):
        return False, f"{nereden} yurt ya da yatakhane: 50/3 otel yatak odasını söyler"
    return True, nereden


def odalar(bina: dict[str, Any]) -> list[dict[str, Any]]:
    """Otel yatak odaları ve süitler."""
    return [s for s in bina.get("spaces") or []
            if _otel_odasi(s.get("name")) and not _var(_sozcukler(s.get("name")), _ODA_DEGIL)]


def oda_uzakliklari(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """50/3: odanın en uzak noktasından kapısına uzaklık; aşan odanın iki kapısı ya da yağmurlamalı otelde 20 m."""
    yagmurlama = bool(bina.get("sprinklers"))
    sinir = float(hal.anahtar["sprinklered"] if yagmurlama else hal.anahtar["distance"])
    bent = "b" if yagmurlama else "a"
    liste = odalar(bina)
    if not liste:
        return [], _satir(hal, y.KAPSAM_DISI, "otel yatak odası ya da süit yok")
    bulgular: list[dict[str, Any]] = []
    kalan = eksik = uzman = 0
    en_uzak = 0.0
    for s in liste:
        ad = s.get("name") or "Oda"
        uzak = s.get("toDoor")
        kapilar = _kapilari(bina, s["uid"])
        if uzak is None:
            eksik += 1
            bulgular.append(_bulgu(
                f"“{ad}”: kapısı çizilmemiş; en uzak noktadan çıkış kapısına uzaklık ölçülemedi",
                "Odanın kapısını duvarına yerleştirin.", hal, [s["uid"]], f"{s['uid']}:kapı", bent, y.EKSIK_VERI))
            continue
        uzak = float(uzak)
        en_uzak = max(en_uzak, uzak)
        if uzak <= sinir:
            continue
        uids = [s["uid"]] + [k["uid"] for k in kapilar]
        if len(kapilar) < 2:
            kalan += 1
            gerekir = ("tamamı yağmurlamalı otelde 20 m'yi aşmamalı" if yagmurlama
                       else "aştığı için birbirinden uzakta en az 2 çıkış kapısı gerekir")
            bulgular.append(_bulgu(
                f"“{ad}”: en uzak noktadan kapısına {_m(uzak)} m > {_m(sinir)} m; {len(kapilar)} kapı, {gerekir}",
                "Odaya öbür kapıdan uzak ikinci bir çıkış kapısı açın ya da en uzak noktayı kapıya yaklaştırın.", hal,
                uids, f"{s['uid']}:uzaklık", bent))
        elif yagmurlama:
            uzman += 1
            bulgular.append(_bulgu(
                f"“{ad}”: en uzak noktadan kapısına {_m(uzak)} m > 20,00 m, {len(kapilar)} kapı; tamamı yağmurlamalı "
                "otelde (b) 20 m'yi aşmamasını ister, (a) aşan odaya iki kapıyı izin verir: birlikte okunuşu idarenin",
                "Uzaklığı ve kapıların yerini belgeleyin; idare karar verir.", hal, uids, f"{s['uid']}:iki-kapı", bent,
                y.UZMAN))
    if kalan:
        satir = _satir(hal, y.KALDI, f"{kalan} oda {_m(sinir)} m'yi aşıyor ve tek kapılı")
    elif uzman:
        satir = _satir(hal, y.UZMAN, f"{uzman} iki kapılı oda 20 m'yi aşıyor")
    elif eksik:
        satir = _satir(hal, y.EKSIK_VERI, f"{eksik} odanın kapısı çizilmemiş")
    else:
        satir = _satir(hal, y.GECTI, f"{len(liste)} odanın en uzak noktası kapısına en çok {_m(en_uzak)} m "
                                     f"({_m(sinir)} m'yi aşan oda iki kapılı; kapıların birbirinden uzaklığı idarenin)")
    return bulgular, satir


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin otel odası bulguları."""
    hal = y.hal("BYKHY", "50/3", gun)
    if hal is None:
        ilk = y.haller("BYKHY", "50/3")[0]
        return y.Sonuc([], [{"rule": "BYKHY 50/3", "title": ilk.baslik, "version": None, "gazette": None,
                             "state": y.UZMAN,
                             "note": f"başvuru günü ({y.yaz(gun)}) kural setinin bildiği ilk hâlden ({ilk.etiket}) önce"}])
    evet, nereden = otel(bina)
    if not evet:
        return y.Sonuc([], [_satir(hal, y.KAPSAM_DISI, nereden)])
    bulgular, satir = oda_uzakliklari(bina, hal)
    return y.Sonuc(bulgular, [satir])


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin otel odası bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "otel odası kuralı")
