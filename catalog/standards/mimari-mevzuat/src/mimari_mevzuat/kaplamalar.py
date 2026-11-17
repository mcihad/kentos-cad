"""Kaplamaların yanıcılığı: Binaların Yangından Korunması Hakkında Yönetmelik 26/2, 26/4 ve Ek-2/Ç (docs/adr/8091).

- **26/2:** döşeme kaplamaları (IFC türü FLOORING) en az normal alevlenici, yüksek binada (4/ccc: bina yüksekliği 21,50
  m'den ya da yapı yüksekliği 30,50 m'den fazla) en az zor alevlenici malzemeden.
- **26/4:** ayrık nizamda müstakil konutlar dışındaki binaların tavan kaplamaları ve asma tavanları (IFC türü CEILING) en
  az zor alevlenici malzemeden. Binanın nizamı çizimde yok: tek konutlu binada (başka bölümü olmayan) tutmayan tavan
  uzman incelemesidir.
- **Ek-2/Ç:** malzemenin beyan edilen TS EN 13501-1 sınıfı (`materials[].reaction`) yanıcılık özelliğine çevrilir: hiç
  yanmaz, zor yanıcı, zor alevlenici, normal alevlenici, kolay alevlenici. Döşeme kaplamasının sınıfı döşeme tablosundan
  (``fl``), öbürlerininki öbür tablodan; sınıf önce tam yazımıyla, yoksa harfiyle eşlenir (harfin en zayıf satırı).

Malzemesi yoksa, malzemenin sınıfı beyan edilmemişse ya da öbür tablonun sınıfıysa eksik veri; yetmiyorsa kaldı.

**Okunan olgular** (``facts.building``): kaplamalar (``coverings``: tür, kat, malzeme, malzemenin sınıfı), katlar, bina ve
yapı yüksekliği, konut ve bölüm sayısı.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import yururluk as y

MADDELER = [("BYKHY", "26/2"), ("BYKHY", "26/4")]


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def ozellik(sinif: str, doseme: bool, tablo: dict[str, Any]) -> tuple[str | None, str | None]:
    """Ek-2/Ç'ye göre ``sinif``'ın yanıcılık özelliği (anahtarı) ve okunmuyorsa nedeni; ``doseme`` döşeme tablosu."""
    satirlar = tablo["floor" if doseme else "other"]
    fl = "fl" in sinif
    if fl != doseme:
        return None, (f"“{sinif}” {'döşeme' if fl else 'döşeme dışı'} malzemesinin sınıfı; {'döşeme' if doseme else 'döşeme dışı'} "
                      f"kaplaması için Ek-2/Ç'nin {'döşeme' if doseme else 'öbür'} tablosunun sınıfı gerekir")
    tam = next((k for s, k in satirlar if s == sinif), None)
    if tam:
        return tam, None
    harf = sinif.split("-")[0]
    kok = next((k for s, k in satirlar if s == harf), None)
    if kok:
        return kok, None
    return None, f"“{sinif}” Ek-2/Ç'nin tablosunda yok"


def _ad(tablo: dict[str, Any], anahtar: str) -> str:
    return dict(tablo["categories"])[anahtar]


def _sira(tablo: dict[str, Any], anahtar: str) -> int:
    return [k for k, _ in tablo["categories"]].index(anahtar)


def _denetle(bina: dict[str, Any], hal: y.Hal, tur: str, en_az: str, ne: str, kod: str, uzman: str | None
             ) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """``tur`` kaplamaların (FLOORING, CEILING) en az ``en_az`` olup olmadığı."""
    tablo = hal.anahtar["ek2c"]
    hepsi = [c for c in bina.get("coverings") or [] if c.get("predefined") == tur]
    if not hepsi:
        return [], _satir(hal, y.KAPSAM_DISI, f"{ne} yok")
    katlar = {k["level"]: k.get("name") or k["level"] for k in bina.get("storeys") or []}
    doseme = tur == "FLOORING"
    istek = _ad(tablo, en_az)
    bulgular = []
    for c in hepsi:
        yer = katlar.get(c.get("level") or "", "Katsız")
        malzeme = c.get("material")
        sinif = c.get("reaction")
        if not malzeme:
            neden, durum = "malzemesi yok", y.EKSIK_VERI
        elif not sinif:
            neden, durum = f"malzemesinin (“{malzeme}”) yanıcılık sınıfı beyan edilmemiş", y.EKSIK_VERI
        else:
            k, sorun = ozellik(sinif, doseme, tablo)
            if k is None:
                neden, durum = sorun, y.EKSIK_VERI
            elif _sira(tablo, k) <= _sira(tablo, en_az):
                continue
            else:
                neden, durum = f"malzemesi “{malzeme}” {sinif}, {_ad(tablo, k)}", uzman and y.UZMAN
        ileti = (f"{yer}: {ne}: {neden}; {hal.madde} en az {istek} ister"
                 + (f" ({uzman})" if durum == y.UZMAN and uzman else ""))
        b = {"code": kod, "message": f"{ileti} — {hal.kisa()}",
             "repair": "Kaplamanın malzemesini seçin ve malzemenin TS EN 13501-1 sınıfını beyan edin (Malzemeler › "
                       "Yanıcılık sınıfı) ya da daha az yanıcı bir malzeme seçin.",
             "uids": [c["uid"]], "rule": hal.kural(), "_iz": f"{c['uid']}:{hal.madde}"}
        if durum:
            b["state"] = durum
        bulgular.append(b)
    if any("state" not in b for b in bulgular):
        satir = _satir(hal, y.KALDI, f"{sum(1 for b in bulgular if 'state' not in b)} {ne} en az {istek} değil")
    elif any(b.get("state") == y.EKSIK_VERI for b in bulgular):
        satir = _satir(hal, y.EKSIK_VERI, f"{ne} malzemesinin yanıcılık sınıfı okunmuyor")
    elif bulgular:
        satir = _satir(hal, y.UZMAN, uzman or "")
    else:
        satir = _satir(hal, y.GECTI, f"{len(hepsi)} {ne} en az {istek}")
    return bulgular, satir


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin kaplama bulguları."""
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    for madde in ("26/2", "26/4"):
        hal = y.hal("BYKHY", madde, gun)
        tur = "FLOORING" if madde == "26/2" else "CEILING"
        if hal is None:
            uids = sorted(c["uid"] for c in bina.get("coverings") or [] if c.get("predefined") == tur)
            if not uids:
                continue
            ilk = y.haller("BYKHY", madde)[0]
            ileti = (f"Başvuru günü ({y.yaz(gun)}) BYKHY {madde} maddesinin kural setindeki ilk hâlinden ({ilk.etiket}) "
                     f"önce: o günün Ek-2 tablosu KentOS'ta yok")
            bulgular.append({"code": "regulation_text_unknown", "message": ileti,
                             "repair": "Kaplamaları maddenin başvuru günündeki metnine göre uzman incelesin.",
                             "uids": uids[:20], "rule": f"BYKHY {madde}", "_iz": f"{madde}:bilinmiyor",
                             "state": y.UZMAN})
            satirlar.append({"rule": f"BYKHY {madde}", "title": ilk.baslik, "version": None, "gazette": None,
                             "state": y.UZMAN, "note": ileti})
            continue
        if madde == "26/2":
            h, s = float(bina.get("height") or 0), float(bina.get("structureHeight") or 0)
            yuksek = h > float(hal.anahtar["building"]) + 1e-9 or s > float(hal.anahtar["structure"]) + 1e-9
            en_az = hal.anahtar["high" if yuksek else "least"]
            b, r = _denetle(bina, hal, tur, en_az, "döşeme kaplaması", "bykhy_floor_covering_reaction", None)
        else:
            konut, bolum = int(bina.get("dwellings") or 0), int(bina.get("units") or 0)
            # Tek konutlu bina ayrık nizamda müstakil konut olabilir: nizam çizimde yok.
            uzman = ("tek konutlu bina: ayrık nizamda müstakil konutsa 26/4 uygulanmaz, binanın nizamı çizimde yok"
                     if konut == 1 and bolum == 1 else None)
            b, r = _denetle(bina, hal, tur, hal.anahtar["least"], "tavan kaplaması", "bykhy_ceiling_reaction", uzman)
        bulgular += b
        satirlar.append(r)
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin kaplama bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "kaplama kuralı")
