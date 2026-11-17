"""Emsalin 22. madde sınırları: Planlı Alanlar İmar Yönetmeliği 22/1-ç, -f, -k, -l, -m ve 44/1 (docs/adr/8079, 8094).

KentOS Emsal ve TAKS tablosunda sınırları sayar (``kentos_domain::zoning``): balkonlar bağımsız bölüm net alanının
%20'sine (22/1-l), toprak altındaki katta depo eklentileri bölüm net alanının %50'sine (22/1-k), kapıcı daireleri toplam
70 m²'ye (22/1-ç), kütüphane 50 m²'ye (22/1-f) kadar dışarıdadır; aşan kısım emsale girer. Bu kural sınırların binanın
bütününe bağlı koşullarına bakar ve sayımı özet satırında söyler:

- **22/1-ç:** yalnız "bu Yönetmelik uyarınca yapılması zorunlu olan" kapıcı dairesi emsal harici sayılır. 44/1'e göre
  katı yakıtlı binada konut bölümü 40'tan (44/1-a), öbür ısıtmada konut kullanımlı binanın bağımsız bölümü 60'tan
  (44/1-b) fazlaysa zorunludur. Konut bölümü 60'tan fazla, ya da 40'tan fazla ve bağımsız bölüm 60'tan fazla: iki hâlde
  de zorunlu, geçti. Konut bölümü yok, ya da konut bölümü 40'ı ve bağımsız bölüm 60'ı geçmiyor: zorunlu değil, kaldı.
  Arası ısıtmaya bağlıdır; çizim ısıtma sistemini söylemez: eksik veri.
- **22/1-f:** kütüphane alışveriş merkezinde ya da 40'tan fazla bağımsız bölümlü binada, normal katta (bodrum, zemin,
  tesisat katı ve çatı arası dışında, 4/1-ııı) ve ortak alan niteliğinde olur; biri tutmuyorsa kaldı. Ortak merdivenle
  bağlantısı, dışarıdan erişiminin olmaması ve ticari kullanıma konu edilmemesi çizimden okunmaz: uzman incelemesi.
- **22/1-k ve 22/1-l:** tablonun sayımı özet satırındadır; bulgusu yoktur.
- **22/1-m** (9.10.2021'den, RG 31623; docs/adr/8094): taşıyıcı sistemi kesme taş yığma bina olarak tasarlanan zemin
  dahil 3 katı geçmeyen geleneksel mimari öğeler barındıran binanın balkonları (bağımsız bölüm net alanının %20'si,
  22/1-l'ninkiyle aynı) ve bahçeyle bağlantılı, gerektiğinde basamakla çıkılan taşlık, dış sofa ve eyvanı katlar alanına
  girmez. Proje binayı böyle beyan ederse tablo Taşlık, Dış sofa ve Eyvan adlı mahalleri emsal harici sayar; bu kural
  koşullarına bakar: bina zemin dahil 3 kattan çoksa ya da mahal zemin katta değilse kaldı; tutuyorsa bahçeyle bağlantısı
  çizimden okunmaz, uzman incelemesi. Bent 9.10.2021'den önce yoktu: daha eski başvuruda kaldı. Satır yalnız böyle beyan
  edilen binada yazılır.

**Okunan olgular** (``facts.building``, KentOS'un ölçtüğü): sınırlı kullanımlar (``limits``: sınır, bağımsız bölüm,
mahaller, alan, sınırın dışarıda bıraktığı, aşan, hepsi ortak alan mı), bağımsız bölüm ve konut bölümü sayısı, katlar
ve mahalleri, binaların kullanımı, taşıyıcı sistem beyanı (``structuralSystem``, ``traditional``).
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import kat_yuksekligi
from . import yururluk as y

MADDELER = [("PAİY", "22/1-ç"), ("PAİY", "22/1-f"), ("PAİY", "22/1-k"), ("PAİY", "22/1-l"), ("PAİY", "22/1-m")]

#: Normal kat sayılmayan katların adlarındaki sözcükler (4/1-ııı): tesisat katı ve çatı arası.
_NORMAL_DISI = ("TESISAT", "CATI")


def _katla(metin: str) -> str:
    """Büyük harf, Türkçe harfler katlanmış (İ, ı → I; Ş → S …): adın sözcükleri karşılaştırılır."""
    tablo = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                           "ş": "S", "Ş": "S", "ü": "U", "Ü": "U", "â": "A", "Â": "A", "î": "I", "Î": "I"})
    return metin.translate(tablo).upper()


def _m2(x: float) -> str:
    """Bir alan iletide: iki ondalık, noktalı (çizimin sayıları gibi)."""
    return f"{x:.2f}"


def _bulgu(code: str, ileti: str, onarim: str, kural: str, uids: list[str], iz: str,
           state: str | None = None) -> dict[str, Any]:
    out: dict[str, Any] = {"code": code, "message": ileti, "repair": onarim, "uids": uids, "rule": kural, "_iz": iz}
    if state:
        out["state"] = state
    return out


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _sinirlar(bina: dict[str, Any], ad: str) -> list[dict[str, Any]]:
    return [s for s in bina.get("limits") or [] if s.get("limit") == ad]


def kapici(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """22/1-ç: emsal harici sayılan kapıcı dairesi 44/1'e göre zorunlu mu."""
    sinirlar = _sinirlar(bina, "caretaker")
    if not sinirlar:
        return [], _satir(hal, y.KAPSAM_DISI, "emsal harici sayılan kapıcı dairesi yok")
    s = sinirlar[0]
    alan, asan, uids = float(s["area"]), float(s["over"]), list(s["rooms"])
    konut, bolum = int(bina.get("dwellings") or 0), int(bina.get("units") or 0)
    az, cok = int(hal.anahtar["few"]), int(hal.anahtar["many"])
    sayim = f"{_m2(alan)} m²" + (f", {_m2(asan)} m²'si 70 m²'yi aştığı için emsale eklendi" if asan > 0 else "")
    sayilar = f"{konut} konut bölümü, {bolum} bağımsız bölüm"
    onarim = ("Kapıcı dairesinin mahallerini emsale dahil sayın (kullanım listesinde emsal sınıfı: Emsale dahil) ya da "
              "adından kapıcıyı kaldırın.")
    if konut > cok or (konut > az and bolum > cok):
        return [], _satir(hal, y.GECTI, f"{sayilar}: 44/1'e göre her ısıtmada zorunlu; {sayim}")
    if konut == 0 or (konut <= az and bolum <= cok):
        neden = ("binada konut bölümü yok" if konut == 0
                 else f"{sayilar}: katı yakıtta 40'tan fazla konut bölümü, öbür ısıtmada 60'tan fazla bağımsız bölüm "
                      "gerekir")
        return ([_bulgu("paiy_caretaker_not_required",
                        f"Kapıcı dairesi emsal harici sayılmış ({_m2(alan)} m²), ama 44/1'e göre zorunlu değil: {neden} "
                        f"— {hal.kisa()}", onarim, hal.kural(), uids, "zorunlu değil")],
                _satir(hal, y.KALDI, f"zorunlu olmayan kapıcı dairesi emsal harici sayılmış ({sayilar})"))
    hangisi = ("katı yakıtlıysa zorunlu (44/1-a), öbür ısıtmada değil (44/1-b 60'tan fazla bağımsız bölüm ister)"
               if konut > az else
               "katı yakıtlı değilse zorunlu (44/1-b), katı yakıtlıysa değil (44/1-a 40'tan fazla konut bölümü ister)")
    return ([_bulgu("paiy_caretaker_not_required",
                    f"Kapıcı dairesi emsal harici sayılmış ({_m2(alan)} m²); {sayilar}: bina {hangisi}. Çizim ısıtma "
                    f"sistemini söylemiyor — {hal.kisa()}",
                    "Isıtma sistemine göre zorunluysa uzman onayıyla işaretleyin; değilse " + onarim[0].lower() + onarim[1:],
                    hal.kural(), uids, "ısıtma", y.EKSIK_VERI)],
            _satir(hal, y.EKSIK_VERI, f"zorunluluğu ısıtma sistemine bağlı ({sayilar})"))


def _normal_disi(bina: dict[str, Any], uids: list[str]) -> list[str]:
    """Mahallerin normal kat olmayan katları, adlarıyla (4/1-ııı): bodrum, zemin, tesisat katı, çatı arası. Zemin kat
    adında Zemin geçen, yoksa kotu ±0,00'a en yakın kattır; altındakiler bodrumdur."""
    katlar = list(bina.get("storeys") or [])
    if not katlar:
        return []
    zemin = next((k for k in katlar if "ZEMIN" in _katla(k.get("name") or "")), None) or min(
        katlar, key=lambda k: abs(float(k.get("elevation") or 0.0)))
    out: list[str] = []
    for k in katlar:
        if not set(uids) & set(k.get("rooms") or []):
            continue
        ad = _katla(k.get("name") or "")
        if (float(k["elevation"]) <= float(zemin["elevation"]) + 1e-9
                or any(w in ad for w in _NORMAL_DISI)):
            if k.get("name") not in out:
                out.append(k.get("name") or k["level"])
    return out


def kutuphane(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """22/1-f: emsal harici sayılan kütüphanenin koşulları."""
    sinirlar = _sinirlar(bina, "library")
    if not sinirlar:
        return [], _satir(hal, y.KAPSAM_DISI, "emsal harici sayılan kütüphane yok")
    s = sinirlar[0]
    alan, asan, uids = float(s["area"]), float(s["over"]), list(s["rooms"])
    bolum = int(bina.get("units") or 0)
    sinir = int(hal.anahtar["units"])
    avm = any("ALISVERIS" in _katla(k) or "AVM" in _katla(k).split() for k in bina.get("uses") or [])
    tutmayan: list[str] = []
    if not avm and bolum <= sinir:
        tutmayan.append(f"bina alışveriş merkezi değil ve {bolum} bağımsız bölümlü (40'tan fazla değil)")
    disi = _normal_disi(bina, uids)
    if disi:
        tutmayan.append("“" + "”, “".join(disi) + "” normal kat değil (bodrum, zemin, tesisat katı ve çatı arası "
                        "dışındaki kat olmalı)")
    if not s.get("common"):
        tutmayan.append("ortak alan değil (bina programında ortak alan grubunda olmalı)")
    sayim = f"{_m2(alan)} m²" + (f", {_m2(asan)} m²'si 50 m²'yi aştığı için emsale eklendi" if asan > 0 else "")
    if tutmayan:
        return ([_bulgu("paiy_library_not_exempt",
                        f"Kütüphane emsal harici sayılmış ({_m2(alan)} m²), ama 22/1-f'nin koşulu tutmuyor: "
                        + "; ".join(tutmayan) + f" — {hal.kisa()}",
                        "Kütüphaneyi emsale dahil sayın (kullanım listesinde emsal sınıfı: Emsale dahil) ya da "
                        "koşullarını sağlayın.", hal.kural(), uids, "koşul: " + "; ".join(tutmayan))],
                _satir(hal, y.KALDI, "kütüphanenin koşulu tutmuyor: " + "; ".join(tutmayan)))
    nerede = "alışveriş merkezi" if avm else f"{bolum} bağımsız bölüm"
    return ([_bulgu("paiy_library_not_exempt",
                    f"Kütüphane emsal harici ({sayim}; {nerede}, normal kat, ortak alan): binanın ortak merdiveniyle "
                    f"bağlantılı olduğu, dışarıdan erişimi olmadığı ve ticari kullanıma konu edilmediği çizimden "
                    f"okunmuyor — {hal.kisa()}",
                    "Kütüphanenin ortak merdivene bağlandığını ve dışarıdan girişi olmadığını uzman incelesin; uygunsa "
                    "uzman onayıyla işaretleyin.", hal.kural(), uids, "okunmuyor", y.UZMAN)],
            _satir(hal, y.UZMAN, "ortak merdivenle bağlantısı ve dışarıdan erişimi çizimden okunmuyor"))


def _sayim(hal: y.Hal, sinirlar: list[dict[str, Any]], ne: str, pay: str) -> dict[str, Any]:
    """22/1-k ve 22/1-l: tablonun sayımı özet satırında."""
    asan = [s for s in sinirlar if float(s["over"]) > 0]
    toplam = sum(float(s["over"]) for s in asan)
    not_ = f"{len(sinirlar)} bağımsız bölümün {ne}"
    if asan:
        not_ += f"; {len(asan)} bölümde {pay}'yi aşan {_m2(toplam)} m² emsale eklendi"
    else:
        not_ += f", hepsi bölüm net alanının {pay}'si içinde"
    return _satir(hal, y.GECTI, not_)


def depo(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """22/1-k: toprak altındaki katta depo eklentileri."""
    sinirlar = _sinirlar(bina, "storage")
    if not sinirlar:
        return [], _satir(hal, y.KAPSAM_DISI, "toprak altındaki katta emsal harici depo eklentisi yok")
    return [], _sayim(hal, sinirlar, "depo eklentisi toprak altındaki katta", "%50")


def balkon(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """22/1-l: bağımsız bölümlerin balkonları."""
    sinirlar = _sinirlar(bina, "balcony")
    if not sinirlar:
        return [], _satir(hal, y.KAPSAM_DISI, "bağımsız bölüme bağlı açık balkon yok")
    return [], _sayim(hal, sinirlar, "balkonu", "%20")


def _zemin_mahalleri(bina: dict[str, Any]) -> set[str]:
    """Zemin katın (±0,00 ve üstündeki katların en altı) ve asma katlarının mahalleri."""
    katlar = kat_yuksekligi.ustteki(bina)
    if not katlar:
        return set()
    zemin = min(katlar, key=lambda k: k["elevation"])
    _, asma = kat_yuksekligi.zemin_yuksekligi(bina, zemin)
    return {u for k in [zemin, *asma] for u in k.get("rooms") or []}


def geleneksel(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """22/1-m: geleneksel kesme taş yığma binanın taşlık, dış sofa ve eyvanının koşulları; balkonları 22/1-l'nin
    %20'siyle sayılır."""
    sinirlar = _sinirlar(bina, "traditional")
    uids = sorted({u for s in sinirlar for u in s["rooms"]})
    alan = sum(float(s["area"]) for s in sinirlar)
    adlar = {m["uid"]: m.get("name") or "" for m in bina.get("spaces") or []}
    onarim = ("Mahali emsale dahil sayın (adını değiştirin ya da kullanım listesinde emsal sınıfı: Emsale dahil) ya da "
              "binayı 22/1-m'nin koşullarına getirin.")
    if hal.anahtar.get("absent"):
        if not uids:
            return [], _satir(hal, y.KAPSAM_DISI, "bent başvuru gününde yok (9.10.2021'de eklendi)")
        ileti = (f"Taşlık, dış sofa ve eyvan emsal harici sayılmış ({_m2(alan)} m²), ama başvuru gününde 22/1-m yok: "
                 f"bent 9.10.2021'de eklendi (RG 31623) — {hal.kisa()}")
        return ([_bulgu("paiy_traditional_not_exempt", ileti, onarim, hal.kural(), uids, "bent yok")],
                _satir(hal, y.KALDI, "taşlık, dış sofa ve eyvan emsal harici sayılmış; bent başvuru gününde yok"))
    balkon = bool(_sinirlar(bina, "balcony"))
    if not uids:
        return [], _satir(hal, y.GECTI if balkon else y.KAPSAM_DISI,
                          "taşlık, dış sofa ya da eyvan yok" + ("; balkonlar 22/1-l'nin %20'siyle sayıldı" if balkon
                                                                else ""))
    n = len(kat_yuksekligi.ustteki(bina))
    en_cok = int(hal.anahtar["storeys"])
    if n > en_cok:
        ileti = (f"Taşlık, dış sofa ve eyvan emsal harici sayılmış ({_m2(alan)} m²), ama bina zemin dahil {n} katlı: "
                 f"22/1-m zemin dahil {en_cok} katı geçmeyen binayı okur — {hal.kisa()}")
        return ([_bulgu("paiy_traditional_not_exempt", ileti, onarim, hal.kural(), uids, f"{n} kat")],
                _satir(hal, y.KALDI, f"bina zemin dahil {n} katlı, en çok {en_cok} olmalı"))
    zemin = _zemin_mahalleri(bina)
    disarda = [u for u in uids if u not in zemin]
    if disarda:
        ne = ", ".join(f"“{adlar.get(u) or u}”" for u in disarda)
        ileti = (f"{ne} zemin katta değil: 22/1-m'nin taşlık, dış sofa ve eyvanı bahçeyle bağlantılı, gerektiğinde "
                 f"basamakla çıkılan, binanın girişiyle ya da çıkma veya balkon altında yer alan alandır — {hal.kisa()}")
        return ([_bulgu("paiy_traditional_not_exempt", ileti, onarim, hal.kural(), disarda, "zemin kat değil")],
                _satir(hal, y.KALDI, f"{len(disarda)} mahal zemin katta değil"))
    ileti = (f"Taşlık, dış sofa ve eyvan emsal harici ({_m2(alan)} m²; zemin dahil {n} katlı geleneksel kesme taş yığma "
             f"bina, zemin kat): bahçeyle bağlantısı ve gerektiğinde basamakla çıkıldığı çizimden okunmuyor — "
             f"{hal.kisa()}")
    return ([_bulgu("paiy_traditional_not_exempt", ileti,
                    "Mahallerin bahçeye bağlandığını uzman incelesin; uygunsa uzman onayıyla işaretleyin.", hal.kural(),
                    uids, "okunmuyor", y.UZMAN)],
            _satir(hal, y.UZMAN, f"{len(uids)} mahal ({_m2(alan)} m²) emsal harici; bahçeyle bağlantısı çizimden "
                                 "okunmuyor"))


_ADLAR = {"22/1-ç": "caretaker", "22/1-f": "library", "22/1-k": "storage", "22/1-l": "balcony",
          "22/1-m": "traditional"}


def _bilinmiyor(madde: str, gun: date, uids: list[str]) -> tuple[dict[str, Any], dict[str, Any]]:
    """Başvuru günü maddenin kural setindeki ilk hâlinden önce: uzman incelemesi."""
    ilk = y.haller("PAİY", madde)[0]
    ileti = (f"Başvuru günü ({y.yaz(gun)}) PAİY {madde} maddesinin kural setindeki ilk hâlinden ({ilk.etiket}) önce: "
             f"o günün metni KentOS'ta yok; emsal tablosu bugünkü hâliyle sayar")
    bulgu = _bulgu("regulation_text_unknown", ileti, "Emsal tablosunu maddenin başvuru günündeki metnine göre uzman "
                   "incelesin.", f"PAİY {madde}", uids, f"{madde}:bilinmiyor", y.UZMAN)
    satir = {"rule": f"PAİY {madde}", "title": ilk.baslik, "version": None, "gazette": None, "state": y.UZMAN,
             "note": ileti}
    return bulgu, satir


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin emsal sınırı bulguları ve maddelerin satırları."""
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    for madde, denetim in (("22/1-ç", kapici), ("22/1-f", kutuphane), ("22/1-k", depo), ("22/1-l", balkon)):
        hal = y.hal("PAİY", madde, gun)
        if hal is None:
            sinirlar = _sinirlar(bina, _ADLAR[madde])
            if not sinirlar:
                continue
            uids = sorted({u for s in sinirlar for u in s["rooms"]})
            b, s = _bilinmiyor(madde, gun, uids)
            bulgular.append(b)
            satirlar.append(s)
            continue
        b, s = denetim(bina, hal)
        bulgular += b
        satirlar.append(s)
    # 22/1-m yalnız geleneksel kesme taş yığma diye beyan edilen binada (docs/adr/8094).
    if kat_yuksekligi.geleneksel(bina) or _sinirlar(bina, "traditional"):
        hal = y.hal("PAİY", "22/1-m", gun)
        if hal is None:
            uids = sorted({u for s in _sinirlar(bina, "traditional") for u in s["rooms"]})
            if uids:
                b, s = _bilinmiyor("22/1-m", gun, uids)
                bulgular.append(b)
                satirlar.append(s)
        else:
            b, s = geleneksel(bina, hal)
            bulgular += b
            satirlar.append(s)
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin emsal sınırı bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    sonuc = y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "emsal sınırı kuralı")
    # Güne bağlı sonucun bulgusu, aday günlerin bulgularının mahallerini seçer.
    for b in sonuc.bulgular:
        if b["code"] == "regulation_date_needed" and not b["uids"]:
            b["uids"] = sorted({u for g in y.adaylar(MADDELER) for f in kural(bina, g).bulgular for u in f["uids"]})[:20]
    return sonuc
