"""Çatıların yanıcılığı: Binaların Yangından Korunması Hakkında Yönetmelik 28/2, 28/3 ve Ek-2/Ç (docs/adr/8134, 8139).

- **28/2:** çatı kaplamaları BROOF sınıfı malzemeden (TS EN 13501-5; malzemenin beyan edilen ``roofClass``'ı); çatı
  kaplamalarının altındaki yüzey (kaplama tahtası) ve yalıtım (su ve ısı yalıtımı) en az zor alevlenici. Kaplama yanmaz
  malzemedense üzerine uygulandığı yüzey en az normal alevlenici olabilir.
- **28/3-b:** yüksek binada (4/ccc: bina yüksekliği 21,50 m'den ya da yapı yüksekliği 30,50 m'den fazla) ve bitişik nizamda
  çatı taşıyıcı sistemi (mertek) ve çatı kaplaması yanmaz malzemeden. Binanın nizamı çizimde yok: yüksek olmayan binada
  yanmaz olmayan çatı uzman incelemesidir (bitişik nizamdaysa kalır).
- **28/3-a (docs/adr/8139):** yüksek binada (ve bitişik nizamda) çatının oturduğu döşeme yatay yangın kesici niteliğinde:
  çatının altında, üstü katının üstünde döşeme olmalı ve sınıfı en az EI 30 (Ek-3/B 10, yangın kesici). Yüksek binada
  döşeme çizilmemişse ya da sınıfı beyan edilmemişse eksik veri, az ise kaldı; yüksek olmayan binada tutmayan çatı uzman
  incelemesi (nizam çizimde yok). Döşemenin kendi Ek-3/B 3 dayanımı ``dayanim``'dadır.
- **Yanmaz:** yönetmelik tanımlamaz; Ek-2/Ç'nin hiç yanmaz (A1) satırı yanmazdır. Zor yanıcı (A2-s1,d0) yanmaz sayılıp
  sayılmayacağı yazılı değil: sonucu ona bağlı çatı uzman incelemesidir.
- **Ek-2/Ç:** katmanın malzemesinin TS EN 13501-1 sınıfı yanıcılık özelliğine ``kaplamalar.ozellik`` ile çevrilir (döşeme
  dışı tablo).

Malzemesi yoksa ya da sınıfı beyan edilmemişse eksik veri; yetmiyorsa kaldı. Çatısı olmayan bina kapsam dışı.

**Okunan olgular** (``facts.building``): çatılar (``roofs``: kat, katmanlar üstten alta, her birinin türü, malzemesi,
malzemenin yanıcılık ve dış yangın sınıfı, oturduğu döşemeler), elemanlar (``elements``: döşemelerin sınıfı), katlar,
bina ve yapı yüksekliği.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import yururluk as y
from .kaplamalar import ozellik
from .yangin_sinifi import okuma

MADDELER = [("BYKHY", "28/2"), ("BYKHY", "28/3")]

#: Kaplamanın altında 28/2'nin saydığı katmanlar: yüzey (kaplama tahtası) ve yalıtımlar.
YUZEY = ("boarding",)
YALITIM = ("waterproofing", "insulation")
#: Katmanın Türkçe adı (Öznitelikler'deki gibi).
AD = {"covering": "kaplama", "battens": "lata", "waterproofing": "su yalıtımı", "boarding": "kaplama tahtası",
      "insulation": "ısı yalıtımı", "rafters": "mertek"}


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _sira(tablo: dict[str, Any], anahtar: str) -> int:
    return [k for k, _ in tablo["categories"]].index(anahtar)


def _ad(tablo: dict[str, Any], anahtar: str) -> str:
    return dict(tablo["categories"])[anahtar]


def _bulgu(hal: y.Hal, kod: str, ileti: str, onarim: str, uid: str, durum: str | None, iz: str) -> dict[str, Any]:
    b = {"code": kod, "message": f"{ileti} — {hal.kisa()}", "repair": onarim, "uids": [uid], "rule": hal.kural(),
         "_iz": iz}
    if durum:
        b["state"] = durum
    return b


def _ozellik(katman: dict[str, Any], tablo: dict[str, Any]) -> tuple[str | None, str]:
    """Katmanın yanıcılık özelliği (anahtarı) ve okunmuyorsa nedeni."""
    malzeme = katman.get("material")
    if not malzeme:
        return None, "malzemesi yok"
    sinif = katman.get("reaction")
    if not sinif:
        return None, f"malzemesinin (“{malzeme}”) yanıcılık sınıfı beyan edilmemiş"
    k, sorun = ozellik(sinif, False, tablo)
    return (k, "") if k else (None, sorun or "")


def _yer(bina: dict[str, Any], cati: dict[str, Any]) -> str:
    katlar = {k["level"]: k.get("name") or k["level"] for k in bina.get("storeys") or []}
    return katlar.get(cati.get("level") or "", "Katsız")


def _yuksek(bina: dict[str, Any], hal: y.Hal) -> bool:
    h, s = float(bina.get("height") or 0), float(bina.get("structureHeight") or 0)
    return h > float(hal.anahtar["building"]) + 1e-9 or s > float(hal.anahtar["structure"]) + 1e-9


def _madde_2(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """28/2: kaplama BROOF; altındaki yüzey ve yalıtım en az zor alevlenici (yanmaz kaplamanın yüzeyi normal)."""
    tablo = hal.anahtar["ek2c"]
    catilar = bina.get("roofs") or []
    if not catilar:
        return [], _satir(hal, y.KAPSAM_DISI, "çatı yok")
    bulgular = []
    onarim_sinif = ("Kaplamanın malzemesini seçin ve TS EN 13501-5 dış yangın sınıfını beyan edin (Malzemeler › Dış "
                    "yangın sınıfı) ya da BROOF sınıflı bir kaplama seçin.")
    onarim_alt = ("Katmanın malzemesini seçin ve TS EN 13501-1 sınıfını beyan edin (Malzemeler › Yanıcılık sınıfı) ya da "
                  "daha az yanıcı bir malzeme seçin.")
    for cati in catilar:
        yer, uid = _yer(bina, cati), cati["uid"]
        katmanlar = cati.get("layers") or []
        kaplama = next((k for k in katmanlar if k.get("kind") == "covering"), None)
        if kaplama is None:
            bulgular.append(_bulgu(hal, "bykhy_roof_covering_class", f"{yer}: çatının kaplama katmanı yok; 28/2 kaplamanın "
                                   "BROOF olmasını ister", "Çatının katmanlarına kaplamayı ekleyin.", uid, y.EKSIK_VERI,
                                   f"{uid}:28/2:kaplama"))
        else:
            malzeme, sinif = kaplama.get("material"), kaplama.get("roofClass")
            if not malzeme:
                neden, durum = "kaplamasının malzemesi yok", y.EKSIK_VERI
            elif not sinif:
                neden, durum = f"kaplamasının malzemesinin (“{malzeme}”) dış yangın sınıfı beyan edilmemiş", y.EKSIK_VERI
            elif sinif.startswith("BROOF"):
                neden, durum = None, None
            else:
                neden, durum = f"kaplaması “{malzeme}” {sinif}", ""
            if neden is not None:
                bulgular.append(_bulgu(hal, "bykhy_roof_covering_class", f"{yer}: çatının {neden}; 28/2 BROOF ister",
                                       onarim_sinif, uid, durum or None, f"{uid}:28/2:kaplama"))
        # Kaplamanın yanmazlığı yüzeyin isteğini düşürür: hiç yanmaz kesin, zor yanıcı belirsiz, okunmayan bilinmez.
        k_ozellik = _ozellik(kaplama, tablo)[0] if kaplama else None
        alt = katmanlar[katmanlar.index(kaplama) + 1:] if kaplama else katmanlar
        for katman in alt:
            tur = katman.get("kind")
            if tur not in YUZEY + YALITIM:
                continue
            ad = AD.get(tur, tur)
            k, sorun = _ozellik(katman, tablo)
            en_az = "zor_alevlenici"
            if k is None:
                bulgular.append(_bulgu(hal, "bykhy_roof_underlay_reaction", f"{yer}: çatının {ad} katmanının {sorun}; 28/2 "
                                       f"en az {_ad(tablo, en_az)} ister", onarim_alt, uid, y.EKSIK_VERI,
                                       f"{uid}:28/2:{tur}"))
                continue
            if _sira(tablo, k) <= _sira(tablo, en_az):
                continue
            normal_yeter = tur in YUZEY and _sira(tablo, k) <= _sira(tablo, "normal_alevlenici")
            ileti = f"{yer}: çatının {ad} katmanı “{katman['material']}” {katman['reaction']}, {_ad(tablo, k)}"
            if normal_yeter and k_ozellik == "hic_yanmaz":
                continue
            if normal_yeter and k_ozellik == "zor_yanici":
                bulgular.append(_bulgu(hal, "bykhy_roof_underlay_reaction", f"{ileti}; kaplama zor yanıcıysa yanmaz sayılıp "
                                       "sayılmadığı yönetmelikte yazılı değil: yanmazsa normal alevlenici yeter, değilse "
                                       "28/2 en az zor alevlenici ister", onarim_alt, uid, y.UZMAN, f"{uid}:28/2:{tur}"))
            elif normal_yeter and k_ozellik is None:
                bulgular.append(_bulgu(hal, "bykhy_roof_underlay_reaction", f"{ileti}; kaplamanın yanıcılık sınıfı okunmuyor: "
                                       "kaplama yanmazsa normal alevlenici yeter, değilse 28/2 en az zor alevlenici ister",
                                       onarim_alt, uid, y.EKSIK_VERI, f"{uid}:28/2:{tur}"))
            else:
                bulgular.append(_bulgu(hal, "bykhy_roof_underlay_reaction", f"{ileti}; 28/2 en az {_ad(tablo, en_az)} ister",
                                       onarim_alt, uid, None, f"{uid}:28/2:{tur}"))
    return bulgular, _ozet(hal, bulgular, f"{len(catilar)} çatının kaplaması BROOF, altındaki katmanlar yeterli",
                           "çatı kaplaması ya da altındaki katmanlar yetmiyor")


def _oturdugu(bina: dict[str, Any], cati: dict[str, Any], hal: y.Hal, yuksek: bool, yer: str) -> dict[str, Any] | None:
    """28/3-a (docs/adr/8139): çatının oturduğu döşeme yatay yangın kesici, en az EI 30; tutuyorsa yok."""
    dk = int(hal.anahtar["seat"])
    siniflar = {e["uid"]: e.get("fireRating") for e in bina.get("elements") or []}
    oturdugu = list(cati.get("seat") or [])
    if not oturdugu:
        neden, durum = "altında, katının üstünde döşeme çizilmemiş", y.EKSIK_VERI if yuksek else y.UZMAN
    else:
        okunan = [(u, siniflar.get(u), okuma(siniflar.get(u))) for u in oturdugu]
        az = [(u, b) for u, b, o in okunan if o is not None and not ("E" in o["harf"] and "I" in o["harf"]
                                                                    and o["dakika"] >= dk)]
        yok = [u for u, b, o in okunan if o is None]
        if az:
            neden = "oturduğu döşeme " + ", ".join(b for _, b in az) + f", en az EI {dk} değil"
            durum = None if yuksek else y.UZMAN
        elif yok:
            neden, durum = "oturduğu döşemenin yangın sınıfı beyan edilmemiş", y.EKSIK_VERI if yuksek else y.UZMAN
        else:
            return None
    if durum == y.UZMAN:
        neden += "; bina yüksek değil, bitişik nizamdaysa 28/3-a uygulanır, nizam çizimde yok"
    kim = "yüksek binada" if yuksek else "bitişik nizamda"
    return _bulgu(hal, "bykhy_roof_seat_fire_stop",
                  f"{yer}: çatının {neden}; 28/3-a {kim} çatının oturduğu döşemeyi yatay yangın kesici (en az EI {dk}) "
                  f"ister", f"Çatının altına, katının üstüne döşeme çizin ve sınıfını en az EI {dk} beyan edin "
                  f"(Öznitelikler › Yangın dayanımı).", cati["uid"], durum, f"{cati['uid']}:28/3-a")


def _madde_3(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """28/3: yüksek binada (ve bitişik nizamda) çatının oturduğu döşeme yatay yangın kesici (a), çatı taşıyıcı sistemi ve
    kaplaması yanmaz (b)."""
    tablo = hal.anahtar["ek2c"]
    catilar = bina.get("roofs") or []
    if not catilar:
        return [], _satir(hal, y.KAPSAM_DISI, "çatı yok")
    yuksek = _yuksek(bina, hal)
    bulgular = []
    for cati in catilar:
        b = _oturdugu(bina, cati, hal, yuksek, _yer(bina, cati))
        if b is not None:
            bulgular.append(b)
    onarim = ("Çatının kaplamasını ve taşıyıcısını hiç yanmaz (A1) malzemeden seçin ya da malzemenin TS EN 13501-1 "
              "sınıfını beyan edin (Malzemeler › Yanıcılık sınıfı).")
    for cati in catilar:
        yer, uid = _yer(bina, cati), cati["uid"]
        katmanlar = cati.get("layers") or []
        # Katmansız çatı tek kaplamadır: hem kaplama hem taşıyıcı.
        istenen = [k for k in katmanlar if k.get("kind") in ("covering", "rafters")]
        sorunlar: list[tuple[str, str | None]] = []
        for katman in istenen:
            ad = AD.get(katman.get("kind"), katman.get("kind"))
            k, sorun = _ozellik(katman, tablo)
            if k == "hic_yanmaz":
                continue
            if k is None:
                if yuksek:
                    sorunlar.append((f"{ad} katmanının {sorun}", y.EKSIK_VERI))
                continue
            ileti = f"{ad} “{katman['material']}” {katman['reaction']}, {_ad(tablo, k)}"
            if k == "zor_yanici":
                sorunlar.append((f"{ileti}: zor yanıcının yanmaz sayılıp sayılmadığı yönetmelikte yazılı değil", y.UZMAN))
            elif yuksek:
                sorunlar.append((ileti, None))
            else:
                sorunlar.append((f"{ileti}: bina yüksek değil, bitişik nizamdaysa kalır; nizam çizimde yok", y.UZMAN))
        if not sorunlar:
            continue
        durum = (None if any(d is None for _, d in sorunlar) else
                 y.EKSIK_VERI if any(d == y.EKSIK_VERI for _, d in sorunlar) else y.UZMAN)
        neden = "; ".join(s for s, _ in sorunlar)
        kim = "yüksek binada" if yuksek else "bitişik nizamda"
        bulgular.append(_bulgu(hal, "bykhy_roof_noncombustible", f"{yer}: çatının {neden}; 28/3-b {kim} çatının taşıyıcı "
                               "sistemini ve kaplamasını yanmaz ister", onarim, uid, durum, f"{uid}:28/3"))
    if not yuksek and not bulgular:
        return [], _satir(hal, y.KAPSAM_DISI, "bina yüksek değil; bitişik nizamda da uygulanır, çatılar hiç yanmaz ve "
                                              "döşemeye oturuyor")
    return bulgular, _ozet(hal, bulgular, f"{len(catilar)} çatının taşıyıcısı ve kaplaması hiç yanmaz, oturduğu döşeme "
                                          f"en az EI {hal.anahtar['seat']}",
                           "çatının oturduğu döşeme, taşıyıcısı ya da kaplaması 28/3'ü tutmuyor")


def _ozet(hal: y.Hal, bulgular: list[dict[str, Any]], tamam: str, kalan: str) -> dict[str, Any]:
    if any("state" not in b for b in bulgular):
        return _satir(hal, y.KALDI, kalan)
    if any(b.get("state") == y.EKSIK_VERI for b in bulgular):
        return _satir(hal, y.EKSIK_VERI, "katmanların ya da oturduğu döşemenin malzemesi, sınıfı okunmuyor")
    if bulgular:
        return _satir(hal, y.UZMAN, "yanmazlık ya da binanın nizamı uzman incelemesi")
    return _satir(hal, y.GECTI, tamam)


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin çatı bulguları."""
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    for madde, denetle in (("28/2", _madde_2), ("28/3", _madde_3)):
        hal = y.hal("BYKHY", madde, gun)
        if hal is None:
            uids = sorted(c["uid"] for c in bina.get("roofs") or [])
            if not uids:
                continue
            ilk = y.haller("BYKHY", madde)[0]
            ileti = (f"Başvuru günü ({y.yaz(gun)}) BYKHY {madde} maddesinin kural setindeki ilk hâlinden ({ilk.etiket}) "
                     f"önce: o günün metni KentOS'ta yok")
            bulgular.append({"code": "regulation_text_unknown", "message": ileti,
                             "repair": "Çatıları maddenin başvuru günündeki metnine göre uzman incelesin.",
                             "uids": uids[:20], "rule": f"BYKHY {madde}", "_iz": f"{madde}:bilinmiyor",
                             "state": y.UZMAN})
            satirlar.append({"rule": f"BYKHY {madde}", "title": ilk.baslik, "version": None, "gazette": None,
                             "state": y.UZMAN, "note": ileti})
            continue
        b, r = denetle(bina, hal)
        bulgular += b
        satirlar.append(r)
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin çatı bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "çatı kuralı")
