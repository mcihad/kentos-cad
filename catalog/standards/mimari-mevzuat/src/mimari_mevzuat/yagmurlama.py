"""Otomatik yağmurlama sisteminin zorunlu olduğu binalar: Binaların Yangından Korunması Hakkında Yönetmelik 96/2
(docs/adr/8108; plan/bim-plan.md B22).

Proje yağmurlama sistemini beyan eder (``settings.arch.sprinklers``, olgularda ``sprinklers``); beyan varsa madde
geçer. Yoksa bentler sırayla okunur, gerektiren her bent bulgudur (``bykhy_sprinklers_required``):

- **a)** yapı yüksekliği 30,50 m'den fazla, konut haricindeki bina (karma bina konut harici sayılır: sıkı olan);
- **b)** yapı yüksekliği 51,50 m'yi geçen konut (bütün bölümleri konut olan bina);
- **c)** alanlarının toplamı 600 m²'den büyük kapalı otopark (adı Otopark ya da Garaj olan mahallerin alanı; araç
  asansörü modelde yok);
- **ç)** birden fazla katlı binada yatılan oda sayısı 100'ü ya da yatak sayısı 200'ü geçen otel, yurt, pansiyon,
  misafirhane; yapı yüksekliği 21,50 m'den fazla bütün yataklı tesis (konaklama ve sağlık). Yatak sayısı çizimde yok:
  odası 100'ü geçmeyen katlı konaklama yapısı eksik veri;
- **d)** toplam alanı 2000 m²'nin üzerinde olan katlı mağaza, alışveriş, ticaret ve eğlence yeri (mahallerin alanlarının
  toplamı);
- **e)** toplam alanı 1000 m²'den fazla, kolay alevlenici ve parlayıcı madde üretilen ya da bulundurulan yapı (17'nin
  saydıkları adından: LPG, akaryakıt, barut, dinamit …); ürettiği maddeyi çizim söylemez: bulgu uzman incelemesi.

Alanlar olguların mahal alanlarıdır: yönetmeliğin alanlarının ölçtüğü dış kontur (PAİY 4/1-ğ-1, brüt). Kullanım bina
programından, yoksa mahal adlarından okunur (``kullanim.siniflar``'ın kuralıyla).
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import yururluk as y
from .kullanim import _otel_odasi, _sozcukler, _var

MADDELER = [("BYKHY", "96/2")]

_KONUT = ("KONUT", "DAIRE", "MESKEN", "APARTMAN", "VILLA")
_KONAKLAMA = ("OTEL", "MOTEL", "PANSIYON", "YURT", "YURD", "MISAFIRHANE", "KONAKLAMA", "YATAKHANE")
_SAGLIK = ("HASTANE", "HUZUREVI", "BAKIMEVI", "SAGLIK", "KLINIK")
_TICARET = ("MAGAZA", "DUKKAN", "ALISVERIS", "AVM", "CARSI", "PASAJ", "MARKET", "TICARET")
_EGLENCE = ("EGLENCE", "SINEMA", "TIYATRO", "GAZINO", "PAVYON", "TAVERNA", "BAR", "DISKOTEK", "DUGUN")
#: 17'nin yüksek tehlikeli yerleri: parlayıcı ve patlayıcı gazlar, patlayıcı maddeler, yanıcı sıvılar.
_PARLAYICI = ("LPG", "AKARYAKIT", "BARUT", "DINAMIT", "MERMI", "PATLAYICI", "PARLAYICI")
_OTOPARK = ("OTOPARK", "GARAJ")


def _m(x: float) -> str:
    return f"{x:.2f}".replace(".", ",")


def _m2(x: float) -> str:
    return f"{x:.0f}"


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _bulgu(bent: str, ileti: str, hal: y.Hal, uids: list[str], state: str | None = None) -> dict[str, Any]:
    out: dict[str, Any] = {
        "code": "bykhy_sprinklers_required",
        "message": f"{ileti} — {hal.kisa(bent)}",
        "repair": "Binaya otomatik yağmurlama sistemi kurun ve Proje ayarları › Mimari › Yangın güvenliği'nde beyan edin.",
        "uids": uids,
        "rule": hal.kural(bent),
        "_iz": f"96/2-{bent}",
    }
    if state:
        out["state"] = state
    return out


def _kullanim(bina: dict[str, Any]) -> list[list[str]]:
    """Binanın kullanımlarının sözcükleri: bina programından; yoksa mahallerin adlarından."""
    if bina.get("uses"):
        return [_sozcukler(u) for u in bina["uses"]]
    return [_sozcukler(s.get("name")) for s in bina.get("spaces") or []]


def konut_binasi(bina: dict[str, Any]) -> bool:
    """Bütün bölümleri konut olan bina: program yalnız konut der, ya da konut sayısı bölüm sayısına eşit."""
    if bina.get("uses"):
        return all(_var(_sozcukler(u), _KONUT) for u in bina["uses"])
    konut, bolum = int(bina.get("dwellings") or 0), int(bina.get("units") or 0)
    return konut > 0 and konut == bolum


def bentler(bina: dict[str, Any], hal: y.Hal) -> list[dict[str, Any]]:
    """96/2'nin bu binada yağmurlama isteyen bentleri, beyana bakmadan: her birinin bendi, sözü, nesneleri, durumu."""
    a = hal.anahtar
    yukseklik = float(bina.get("structureHeight") or 0.0)
    katli = len(bina.get("storeys") or []) > 1
    mahaller = bina.get("spaces") or []
    kullanim = _kullanim(bina)
    konut = konut_binasi(bina)
    out: list[dict[str, Any]] = []
    if not konut and yukseklik > float(a["height"]):
        out.append({"bent": "a", "ileti": f"konut haricindeki bina, yapı yüksekliği {_m(yukseklik)} m > 30,50 m",
                    "uids": []})
    if konut and yukseklik > float(a["dwellingHeight"]):
        out.append({"bent": "b", "ileti": f"konut, yapı yüksekliği {_m(yukseklik)} m > 51,50 m", "uids": []})
    otopark = [s for s in mahaller if _var(_sozcukler(s.get("name")), _OTOPARK)]
    alan = sum(float(s.get("area") or 0.0) for s in otopark)
    if alan > float(a["parking"]):
        out.append({"bent": "c", "ileti": f"kapalı otoparkların alanı {_m2(alan)} m² > 600 m²",
                    "uids": [s["uid"] for s in otopark]})
    konaklama = any(_var(w, _KONAKLAMA) for w in kullanim)
    saglik = any(_var(w, _SAGLIK) for w in kullanim)
    if (konaklama or saglik) and yukseklik > float(a["bedsHeight"]):
        out.append({"bent": "ç", "ileti": f"yataklı tesis, yapı yüksekliği {_m(yukseklik)} m > 21,50 m", "uids": []})
    elif konaklama and katli:
        odalar = [s for s in mahaller if _otel_odasi(s.get("name"))]
        if len(odalar) > int(a["rooms"]):
            out.append({"bent": "ç", "ileti": f"katlı konaklama yapısı, {len(odalar)} yatılan oda > 100",
                        "uids": [s["uid"] for s in odalar]})
        else:
            out.append({"bent": "ç", "ileti": f"katlı konaklama yapısı, {len(odalar)} yatılan oda; yatak sayısı 200'ü "
                                              "geçerse de gerekir, yatak sayısı çizimde yok",
                        "uids": [s["uid"] for s in odalar], "state": y.EKSIK_VERI})
    # d) Yalnız ticaret ve eğlence binasında bütün mahaller; karma binada ticaret ve eğlence adlı mahaller, katlı
    # olması onların katlarından.
    toplam = sum(float(s.get("area") or 0.0) for s in mahaller)
    ticari = lambda w: _var(w, _TICARET) or _var(w, _EGLENCE)  # noqa: E731
    if kullanim and all(ticari(w) for w in kullanim if w) and any(kullanim):
        dukkan, dukkan_katli = toplam, katli
    else:
        secilen = [s for s in mahaller if ticari(_sozcukler(s.get("name")))]
        dukkan = sum(float(s.get("area") or 0.0) for s in secilen)
        dukkan_katli = len({s.get("level") for s in secilen}) > 1
    if dukkan_katli and dukkan > float(a["shops"]):
        out.append({"bent": "d", "ileti": f"katlı mağaza, alışveriş, ticaret ya da eğlence yeri, mahallerin alanı "
                                          f"{_m2(dukkan)} m² > 2000 m²", "uids": []})
    parlayici = [s for s in mahaller if _var(_sozcukler(s.get("name")), _PARLAYICI)]
    if (parlayici or any(_var(w, _PARLAYICI) for w in kullanim)) and toplam > float(a["flammable"]):
        out.append({"bent": "e", "ileti": f"kolay alevlenici ve parlayıcı madde bulunan yapı, mahallerin alanı "
                                          f"{_m2(toplam)} m² > 1000 m²; ürettiği ya da bulundurduğu madde çizimde yok",
                    "uids": [s["uid"] for s in parlayici], "state": y.UZMAN})
    return out


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin yağmurlama bulguları."""
    hal = y.hal("BYKHY", "96/2", gun)
    if hal is None:
        ilk = y.haller("BYKHY", "96/2")[0]
        return y.Sonuc([], [{"rule": "BYKHY 96/2", "title": ilk.baslik, "version": None, "gazette": None,
                             "state": y.UZMAN,
                             "note": f"başvuru günü ({y.yaz(gun)}) kural setinin bildiği ilk hâlden ({ilk.etiket}) önce"}])
    gereken = bentler(bina, hal)
    if not gereken:
        return y.Sonuc([], [_satir(hal, y.KAPSAM_DISI, "hiçbir bent yağmurlama istemiyor")])
    adlar = ", ".join(g["bent"] for g in gereken)
    if bina.get("sprinklers"):
        return y.Sonuc([], [_satir(hal, y.GECTI, f"yağmurlama sistemi beyan edilmiş ({adlar} bendi istiyor)")])
    bulgular = [_bulgu(g["bent"], f"Otomatik yağmurlama sistemi gerekir: {g['ileti']}; projede beyan edilmemiş", hal,
                       g["uids"], g.get("state")) for g in gereken]
    if any(not g.get("state") for g in gereken):
        durum = y.KALDI
    elif any(g.get("state") == y.EKSIK_VERI for g in gereken):
        durum = y.EKSIK_VERI
    else:
        durum = y.UZMAN
    return y.Sonuc(bulgular, [_satir(hal, durum, f"{adlar} bendi yağmurlama istiyor, beyan yok")])


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin yağmurlama bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "yağmurlama kuralı")

