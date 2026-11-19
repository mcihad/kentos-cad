"""Kaçış yolu kapıları: Binaların Yangından Korunması Hakkında Yönetmelik 47/2 ve 47/3 (docs/adr/8078 §8).

- **47/2:** kullanıcı yükü 50 kişiyi aşan mekânın çıkış kapıları kaçış yönüne doğru açılır. Mekânın çıkış kapısı
  sınırındaki kapıdır; kaçış yönü mekândan dışarıdır: kanadı mekânın içine açılan döner kanatlı kapı kalır. Sürgülü,
  katlanır ya da başka işleyişli kapının "kaçış yönüne açılması" metinde okunmaz: uzman incelemesi.
- **47/3:** kaçış merdiveni ve yangın güvenlik holü kapıları duman sızdırmaz ve kendiliğinden kapanır; merdiven 4'ten
  az kata hizmet veriyorsa en az 60, bodrum katlara ya da 4'ten fazla kata hizmet veriyorsa en az 90 dakika yangına
  dayanır. Bodrumsuz tam 4 kat metinde yok: uzman incelemesi. Kapının yangın sınıfı müellifin beyanıdır (kapının ya da
  tipinin, docs/adr/8068): TS EN 13501-2 yazımıyla (``EI2 60-S200C5``) okunur: dakikası sayı, duman sızdırmazlığı
  ``S``, kendiliğinden kapanma ``C``. Beyan yoksa ya da okunamıyorsa eksik veri.

**Okunan olgular** (``facts.building``, KentOS'un ölçtüğü): kullanıcı yükü olan mahaller (kişi, BYKHY Ek-5/A'ya göre
Mimari denetimin hesabı), kapılar (iki yanındaki mahaller, kanadın açıldığı mahal, işleyişi, beyan edilen yangın
sınıfı, kaçış merdiveni ya da yangın güvenlik holüne açılıp açılmadığı) ve kaçış merdiveni olan katlar.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import katman
from . import yururluk as y
from .yangin_sinifi import sinif

MADDELER = [("BYKHY", "47/2"), ("BYKHY", "47/3")]

def _bulgu(code: str, ileti: str, onarim: str, kural: str, uids: list[str], iz: str,
           state: str | None = None) -> dict[str, Any]:
    out: dict[str, Any] = {"code": code, "message": ileti, "repair": onarim, "uids": uids, "rule": kural, "_iz": iz}
    if state:
        out["state"] = state
    return out


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _kisi(n: float) -> str:
    return f"{n:.0f}" if abs(n - round(n)) < 1e-9 else f"{n:.1f}"


def acilis(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """47/2: kalabalık mekânların çıkış kapıları."""
    sinir = float(hal.anahtar.get("persons", 50))
    mekanlar = [m for m in bina.get("rooms") or [] if float(m.get("persons") or 0) > sinir]
    kapilar = bina.get("doors") or []
    bulgular: list[dict[str, Any]] = []
    kalan = uzman = 0
    for m in mekanlar:
        ad = m.get("name") or "Mahal"
        for k in kapilar:
            if m["uid"] not in (k.get("between") or []):
                continue
            uids = [k["uid"], m["uid"]]
            if k.get("operation", "swing") != "swing":
                uzman += 1
                bulgular.append(_bulgu(
                    "bykhy_exit_door_swing",
                    f"“{ad}” ({_kisi(m['persons'])} kişi): çıkış kapısı döner kanatlı değil ({k.get('operation')}), kaçış "
                    f"yönüne açılması okunamıyor — {hal.kisa()}",
                    "Kapının kaçış yönüne açılıp açılmadığına idare karar verir; gerekiyorsa kanatlı kapı kullanın.",
                    hal.kural(), uids, f"{k['uid']}:işleyiş", y.UZMAN))
            elif k.get("opensInto") == m["uid"]:
                kalan += 1
                bulgular.append(_bulgu(
                    "bykhy_exit_door_swing",
                    f"“{ad}” ({_kisi(m['persons'])} kişi, 50'den fazla): çıkış kapısı mekânın içine açılıyor, kaçış "
                    f"yönüne açılmalı — {hal.kisa()}",
                    "Kapının açılış yönünü dışarıya çevirin (Öznitelikler › Açılış yönü).",
                    hal.kural(), uids, f"{k['uid']}:içeri"))
    if not mekanlar:
        satir = _satir(hal, y.KAPSAM_DISI, "kullanıcı yükü 50 kişiyi aşan mekân yok")
    elif kalan:
        satir = _satir(hal, y.KALDI, f"{kalan} çıkış kapısı mekânın içine açılıyor")
    elif uzman:
        satir = _satir(hal, y.UZMAN, f"{uzman} çıkış kapısı döner kanatlı değil")
    else:
        satir = _satir(hal, y.GECTI, f"{len(mekanlar)} kalabalık mekânın çıkış kapıları dışarı açılıyor")
    return bulgular, satir


def dayanim(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """47/3: kaçış merdiveni ve yangın güvenlik holü kapıları."""
    kapilar = [k for k in bina.get("doors") or [] if k.get("stair") or k.get("lobby")]
    katlar = [s for s in bina.get("storeys") or [] if s.get("stair")]
    bodrum = any(katman.kot(s) < -1e-6 for s in katlar)
    n = len(katlar)
    az, cok, sinir = int(hal.anahtar["few"]), int(hal.anahtar["many"]), int(hal.anahtar["storeys"])
    gerek = cok if bodrum or n > sinir else (az if n < sinir else None)
    hizmet = f"merdiven {n} kata hizmet veriyor" + (", bodrum dahil" if bodrum else "")
    bulgular: list[dict[str, Any]] = []
    kalan = eksik = uzman = 0
    for k in kapilar:
        ne = "Yangın güvenlik holü kapısı" if k.get("lobby") and not k.get("stair") else "Kaçış merdiveni kapısı"
        uids = [k["uid"]]
        beyan = k.get("fireRating")
        okunan = sinif(beyan)
        if okunan is None:
            eksik += 1
            neden = f"“{beyan}” okunamadı" if beyan else "yangın sınıfı beyan edilmemiş"
            bulgular.append(_bulgu(
                "bykhy_stair_door_rating",
                f"{ne}: {neden}; {hal.madde} duman sızdırmaz, kendiliğinden kapanan ve dakikası belli kapı ister — "
                f"{hal.kisa()}",
                "Kapının (ya da tipinin) yangın sınıfını TS EN 13501-2 yazımıyla beyan edin (EI2 60-S200C5).",
                hal.kural(), uids, f"{k['uid']}:beyan", y.EKSIK_VERI))
            continue
        eksikler = []
        if gerek is not None and okunan["dakika"] < gerek:
            eksikler.append(f"{okunan['dakika']} dakika < {gerek} dakika ({hizmet})")
        if not okunan["duman"]:
            eksikler.append("duman sızdırmaz değil (S yok)")
        if not okunan["kapanma"]:
            eksikler.append("kendiliğinden kapanmıyor (C yok)")
        if eksikler:
            kalan += 1
            bulgular.append(_bulgu(
                "bykhy_stair_door_rating", f"{ne} “{beyan}”: " + "; ".join(eksikler) + f" — {hal.kisa()}",
                "Kapının (ya da tipinin) yangın sınıfını yönetmeliğin istediği dakika, duman sızdırmazlık ve "
                "kendiliğinden kapanmayla beyan edin.", hal.kural(), uids, f"{k['uid']}:sınıf"))
        elif gerek is None:
            uzman += 1
            bulgular.append(_bulgu(
                "bykhy_stair_door_rating",
                f"{ne} “{beyan}”: {hizmet}; 47/3 4'ten az katta 60, 4'ten fazla katta 90 dakika der, tam 4 katı "
                f"söylemez — {hal.kisa()}", "Tam 4 kata hizmet veren merdivenin kapısına idare karar verir.",
                hal.kural(), uids, f"{k['uid']}:dört", y.UZMAN))
    if not kapilar:
        satir = _satir(hal, y.KAPSAM_DISI, "kaçış merdiveni ya da yangın güvenlik holü kapısı yok")
    elif kalan:
        satir = _satir(hal, y.KALDI, f"{kalan} kapı yetmiyor ({hizmet})")
    elif eksik:
        satir = _satir(hal, y.EKSIK_VERI, f"{eksik} kapının yangın sınıfı beyan edilmemiş ya da okunamadı")
    elif uzman:
        satir = _satir(hal, y.UZMAN, f"{hizmet}: metin tam 4 katı söylemiyor")
    else:
        satir = _satir(hal, y.GECTI, f"{len(kapilar)} kapı yetiyor ({hizmet})")
    return bulgular, satir


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin kapı bulguları."""
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    for madde, denetim in (("47/2", acilis), ("47/3", dayanim)):
        hal = y.hal("BYKHY", madde, gun)
        if hal is None:
            continue
        b, s = denetim(bina, hal)
        bulgular += b
        satirlar.append(s)
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin kaçış yolu kapısı bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "kapı kuralı")
