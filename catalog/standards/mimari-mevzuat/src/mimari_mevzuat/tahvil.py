"""Bina yüksekliği ile kat adedinin birbirine tahvili: Planlı Alanlar İmar Yönetmeliği 9/1-c (docs/adr/8157).

- **9/1-c (1.10.2017):** imar planlarında gösterilen bina yüksekliklerinin ya da kat adetlerinin birbirlerine tahvilinde
  ya da neye karşılık geldiklerinin tespitinde maddenin esasları (9/1-a: 28 inci maddede katın konumuna göre en çok kat
  yükseklikleri ile kat adedinin çarpımı) ve binanın kot aldığı noktaya en çok 1,20 m eklenerek belirlenen subasman kotu
  dikkate alınır. Sivas İmar Yönetmeliği 9/1-c aynıdır (1,20 m).
- **Tahvil (KentOS'un okuması):** n katın yüksekliği H(n) = 1,20 + zemin katın en çok yüksekliği + (n − 1) × normal
  katın en çok yüksekliği. Yükseklikler parselin bölgesinin 28/1 bendinden, başvuru gününün hâliyle ve n katlı binanın
  bendiyle (28/1-c'de bodrum hariç 3 katı geçmeyen binada 4,00 m; 28/1-d'de zemin dahil 3 katı geçmeyen geleneksel
  kesme taş yığma binada 4,50 m); asma katlı zemin kat, karma alanda konut projesinin katları konut katı; planın kat
  yüksekliği notları önce (69/3). Planın bina yüksekliği H'nin kat adedi H(n) ≤ H olan en büyük n'dir. Sivas seçiliyse
  Sivas'ın 28/1'iyle de hesaplanır, sıkı olanı bağlar.
- **Ne zaman:** plan notlarında yalnız biri varsa (``buildingHeight`` ya da ``storeys``). İkisi de varsa tahvil gerekmez
  (kapsam dışı); hiçbiri yoksa satır yok. Planın notu 5/11'le zaten aşılamaz (``bina_yuksekligi``). Bir bina kendi
  notlarını söylüyorsa (docs/adr/8163) her bina kendi notlarıyla (yazmadığı ölçüde projeninkiyle) okunur.
- **Sonuç:** her binanın kat adedi (binasının ±0,00'ından, docs/adr/8154) planın yüksekliğinin tahvilini, yüksekliği
  planın kat adedinin tahvilini aşmıyorsa geçti. Aşıyorsa uzman incelemesi: idareler subasmanı tahvile katıp katmamakta
  ve 28/2'nin (28 inci maddenin kat yükseklikleri dikkate alınmadan bina yüksekliği verilen planlar) okunuşunda ayrışır;
  karar idarenin.
- **Okunamayan:** bölgesi yazılmamış konut dışı bina ve en üst katının yüksekliği yazılmamış bina (kat adedinden
  tahvilde) satırda eksik veri, bulgusuz: nedenin bulgusunu kat yüksekliği kuralı verir. 28/1'in saymadığı kullanım
  (öbür) kapsam dışı.

**Okunan olgular** (``facts.building``): katlar (kot, binanın ±0,00'ı, kattan kata yükseklik, asma kat, adı, binası,
bölgesi, taşıyıcı sistemi, mahalleri), plan notları, bölge, yerel yönetmelik, bağımsız bölümler ve konutlar, taşıyıcı
sistem beyanı.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import bina_yuksekligi, kat_yuksekligi, katman
from . import yururluk as y

MADDE = ("PAİY", "9/1-c")
YUKSEKLIK, KAT = bina_yuksekligi.YUKSEKLIK, bina_yuksekligi.KAT
KOD = "paiy_height_storeys_conversion"
#: Bir binanın bu kadar katından fazlası denenmez.
EN_COK_KAT = 80


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _bent(bina: dict[str, Any], n: int, gun: date) -> str | None:
    """n katlı binanın 28/1 bendi: bölgenin bendi; konut bölgesinde zemin dahil 3 katı geçmeyen geleneksel kesme taş
    yığma binada (bendin günüyse) 28/1-d. Bölgesi yazılmamış konut projesi konut alanıdır."""
    zone = bina.get("zone")
    madde = kat_yuksekligi.BOLGE_MADDESI.get(zone) if zone else ("28/1-c" if kat_yuksekligi.konut(bina) else None)
    if madde in kat_yuksekligi.KONUT_BENTLERI and kat_yuksekligi.geleneksel(bina):
        d = y.hal("PAİY", kat_yuksekligi.GELENEKSEL, gun)
        if d is not None and not d.anahtar.get("absent") and n <= int(d.anahtar["storeys"]):
            return kat_yuksekligi.GELENEKSEL
    return madde


def yukseklik(bina: dict[str, Any], n: int, gun: date, subasman: float, yon: str = "PAİY") -> float | None:
    """n katın tahvildeki yüksekliği: subasman + zemin katın en çok yüksekliği + (n − 1) normal katınki; ``yon``un
    (``PAİY`` ya da ``Sivas``) 28/1'iyle, planın kat yüksekliği notları önce. Okunamıyorsa yok."""
    zemin_notu, kat_notu = kat_yuksekligi.plan_notu(bina, True), kat_yuksekligi.plan_notu(bina, False)
    madde = _bent(bina, n, gun)
    hal = y.hal(yon, madde, gun) if madde else None
    katlar = kat_yuksekligi.ustteki(bina)
    zemin = min(katlar, key=lambda k: k["elevation"]) if katlar else None
    asma = bool(zemin and kat_yuksekligi.zemin_yuksekligi(bina, zemin)[1])
    konut = kat_yuksekligi.konut(bina)

    def en_cok(zemin_mi: bool) -> float | None:
        not_ = zemin_notu if zemin_mi else kat_notu
        if not_ is not None:
            return float(not_["value"])
        return kat_yuksekligi.sinir(hal, n, zemin_mi, asma and zemin_mi, konut).deger if hal is not None else None

    z = en_cok(True)
    k = en_cok(False) if n > 1 else 0.0
    if z is None or k is None:
        return None
    return round(subasman + z + (n - 1) * k, 3)


def kaynak(bina: dict[str, Any], yon: str) -> str:
    """İletide tahvilin kat yüksekliklerinin nereden okunduğu, araç durumunda: planın notları ya da ``yon``un 28/1'i."""
    zemin_notu = kat_yuksekligi.ZEMIN in {n["key"] for n in bina.get("planNotes") or []}
    if kat_yuksekligi.plan_notu(bina, False) is not None:
        return "planın zemin kat ve kat yüksekliği notlarıyla" if zemin_notu else "planın kat yüksekliği notuyla"
    if zemin_notu:
        return f"planın zemin kat notu ve {yon} 28/1'in en çok kat yükseklikleriyle"
    return f"{yon} 28/1'in en çok kat yükseklikleriyle"


def kat_adedi(bina: dict[str, Any], h: float, gun: date, subasman: float, yon: str = "PAİY") -> int | None:
    """Planın ``h`` yüksekliğinin tahvildeki kat adedi: H(n) ≤ h olan en büyük n (hiçbiri değilse 0); okunamıyorsa
    yok."""
    en_iyi = 0
    for n in range(1, EN_COK_KAT + 1):
        hn = yukseklik(bina, n, gun, subasman, yon)
        if hn is None:
            return None
        if hn <= h + katman.PAY:
            en_iyi = n
    return en_iyi


def _binalar(bina: dict[str, Any]) -> list[dict[str, Any]]:
    """Her binanın olguları: katları, bölgesi (kendi bölgesi yoksa projeninki, docs/adr/8144), taşıyıcı sistemi
    (docs/adr/8145) ve plan notları (kendi notları, yazmadığı ölçüde projeninkiler; docs/adr/8163)."""
    out = []
    for katlar in katman.binalara(bina):
        zone = next((k["zone"] for k in katlar if k.get("zone")), None) or bina.get("zone")
        kendi = next((k for k in katlar if k.get("structuralSystem")), None)
        yapi = ({"structuralSystem": kendi["structuralSystem"], "traditional": bool(kendi.get("traditional"))}
                if kendi else {})
        out.append(dict(bina, storeys=katlar, zone=zone, planNotes=katman.plan_notlari(bina, katlar), **yapi))
    return out


def _ad(bina: dict[str, Any], hepsi: int) -> str:
    """İletide binanın adı: birden çok binada bina programındaki adıyla, ad yoksa kimliğiyle (“a” binası); tek binada
    yok."""
    ad = katman.bina_adi(bina.get("storeys") or [])
    return f"{ad}: " if hepsi > 1 and ad else ""


def _notlar(bina: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """Binanın tahvilde okunan plan notları: bina yüksekliği ve kat adedi."""
    return {n["key"]: n for n in bina.get("planNotes") or [] if n["key"] in (YUKSEKLIK, KAT)}


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin tahvil bulguları ve 9/1-c'nin satırı."""
    hal = y.hal(*MADDE, gun)
    hepsi = _binalar(bina)
    if hal is None:
        ilk = y.haller(*MADDE)[0]
        not_ = f"başvuru günü ({y.yaz(gun)}) kural setinin bildiği ilk hâlden ({ilk.etiket}) önce"
        return y.Sonuc([], [{"rule": "PAİY 9/1-c", "title": ilk.baslik, "version": None, "gazette": None,
                             "state": y.UZMAN, "note": not_}])
    if all(len(_notlar(b)) == 2 for b in hepsi):
        return y.Sonuc([], [_satir(hal, y.KAPSAM_DISI, "planın bina yüksekliği de kat adedi de var: tahvil gerekmez")])
    subasman = float(hal.anahtar["subasman"])
    sivas = bina.get("local") == "sivas"
    yonler = ["PAİY", "Sivas"] if sivas else ["PAİY"]
    binalar = [b for b in hepsi if bina_yuksekligi.katlar(b)]
    if not binalar:
        return y.Sonuc([], [_satir(hal, y.KAPSAM_DISI, "±0,00 üstünde kat yok")])
    bulgular: list[dict[str, Any]] = []
    notlar_: list[str] = []
    # Okunamayan binaların nedenleri: bulgusunu kat yüksekliği kuralı verir (bölge, katın yüksekliği); burada satır.
    eksik: list[str] = []
    kapsam = 0
    # Planın iki değeri de olan binalar (docs/adr/8163: binanın kendi notlarıyla): tahvil gerekmez.
    gerekmez: list[str] = []
    for b in binalar:
        katlar = bina_yuksekligi.katlar(b)
        ad = _ad(b, len(binalar))
        notlar = _notlar(b)
        if len(notlar) != 1:
            if notlar:
                gerekmez.append(f"{ad}tahvil gerekmez (planın iki değeri de var)")
            continue
        uids = [u for k in katlar for u in k.get("rooms") or []][:20]
        madde = _bent(b, len(katlar), gun)
        notlu = kat_yuksekligi.plan_notu(b, True) is not None and kat_yuksekligi.plan_notu(b, False) is not None
        if madde is None and not notlu:
            if b.get("zone") == "other":
                kapsam += 1
            else:
                eksik.append(f"{ad}bölge yazılmamış")
            continue
        if YUKSEKLIK in notlar:
            plan = float(notlar[YUKSEKLIK]["value"])
            sayilar = {yon: kat_adedi(b, plan, gun, subasman, yon) for yon in yonler}
            sayilar = {yon: s for yon, s in sayilar.items() if s is not None}
            if not sayilar:
                eksik.append(f"{ad}kat yükseklikleri okunamıyor")
                continue
            yon, izin = min(sayilar.items(), key=lambda x: x[1])
            kac = len(katlar)
            yan = "".join(f"; {o} ile {s} kat" for o, s in sayilar.items() if o != yon and s != izin)
            if kac <= izin:
                notlar_.append(f"{ad}planın {katman.metre(plan)}'si {izin} kata karşılık gelir; binada {kac} kat")
                continue
            # Tahvilin iki komşu değeri: planın yüksekliğine sığan son kat adedi ve sığmayan ilki.
            olcu = [f"{n} kat {katman.metre(h_)}" for n in (izin, izin + 1) if n >= 1
                    and (h_ := yukseklik(b, n, gun, subasman, yon)) is not None]
            bulgular.append({
                "code": KOD,
                "message": f"{ad}Planın bina yüksekliği {katman.metre(plan)}, 9/1-c'nin tahviliyle ({subasman:.2f} m "
                           f"subasman ve {kaynak(b, yon)} {', '.join(olcu)}) {izin} kata karşılık gelir{yan}; "
                           f"binada {kac} kat — {hal.kisa()}",
                "repair": "Kat adedini tahvile göre düzenleyin ya da idarenin tahvil okunuşunu (subasmanın sayılması, "
                          "28/2) belgeleyip uzman onayıyla işaretleyin.",
                "uids": uids, "rule": hal.kural(), "_iz": f"tahvil:kat:{plan}:{izin}", "state": y.UZMAN})
        else:
            plan_kat = int(round(float(notlar[KAT]["value"])))
            ust = katlar[-1]
            if ust.get("height") is None:
                eksik.append(f"{ad}“{ust['name']}” katının yüksekliği yazılmamış")
                continue
            boy = round(katman.kot(ust) + float(ust["height"]), 3)
            sinirlar = {yon: yukseklik(b, plan_kat, gun, subasman, yon) for yon in yonler}
            sinirlar = {yon: s for yon, s in sinirlar.items() if s is not None}
            if not sinirlar or plan_kat < 1:
                eksik.append(f"{ad}kat yükseklikleri okunamıyor")
                continue
            yon, izin_h = min(sinirlar.items(), key=lambda x: x[1])
            yan = "".join(f"; {o} ile {katman.metre(s)}" for o, s in sinirlar.items() if o != yon and s != izin_h)
            if boy <= izin_h + katman.PAY:
                notlar_.append(f"{ad}planın {plan_kat} katı {katman.metre(izin_h)}'ye karşılık gelir; bina "
                               f"{katman.metre(boy)}")
                continue
            bulgular.append({
                "code": KOD,
                "message": f"{ad}Planın kat adedi {plan_kat}, 9/1-c'nin tahviliyle ({subasman:.2f} m subasman ve "
                           f"{kaynak(b, yon)}) {katman.metre(izin_h)}'ye karşılık gelir{yan}; binanın yüksekliği "
                           f"{katman.metre(boy)} — {hal.kisa()}",
                "repair": "Kat yüksekliklerini ya da subasmanı tahvile göre düzenleyin ya da idarenin okunuşunu "
                          "belgeleyip uzman onayıyla işaretleyin.",
                "uids": uids, "rule": hal.kural(), "_iz": f"tahvil:yükseklik:{plan_kat}:{izin_h}", "state": y.UZMAN})
    uzman = sum(1 for b in bulgular if b.get("state") == y.UZMAN)
    if uzman:
        return y.Sonuc(bulgular, [_satir(hal, y.UZMAN, f"{uzman} bina planın değerinin tahvilini aşıyor")])
    if eksik:
        return y.Sonuc(bulgular, [_satir(hal, y.EKSIK_VERI, "tahvil okunamadı: " + " · ".join(eksik))])
    if not notlar_ and kapsam:
        return y.Sonuc([], [_satir(hal, y.KAPSAM_DISI, "planın kullanımı 28/1'in saymadığı bir kullanım")])
    if not notlar_:
        ne = ("planın bina yüksekliği de kat adedi de var: tahvil gerekmez" if gerekmez
              else "planın değerini okuyan binada ±0,00 üstünde kat yok")
        return y.Sonuc([], [_satir(hal, y.KAPSAM_DISI, ne)])
    return y.Sonuc([], [_satir(hal, y.GECTI, " · ".join(notlar_ + gerekmez))])


def maddeler(bina: dict[str, Any]) -> list[tuple[str, str]]:
    """Gün bilinmezken denenecek maddeler: 9/1-c ve binaların 28/1 bentleri (28/1-c'nin 12.8.2023 hâli gibi)."""
    out = [MADDE]
    for b in _binalar(bina):
        zone = b.get("zone")
        madde = kat_yuksekligi.BOLGE_MADDESI.get(zone) if zone else ("28/1-c" if kat_yuksekligi.konut(b) else None)
        bentler = [madde] if madde else []
        if madde in kat_yuksekligi.KONUT_BENTLERI and kat_yuksekligi.geleneksel(b):
            bentler.append(kat_yuksekligi.GELENEKSEL)
        out += [("PAİY", m) for m in bentler if ("PAİY", m) not in out]
    return out


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin tahvil bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    notlar = {n["key"] for b in _binalar(bina) for n in b.get("planNotes") or []}
    if not bina.get("storeys") or not (YUKSEKLIK in notlar or KAT in notlar):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), maddeler(bina), gun, neden, "tahvil kuralı")
