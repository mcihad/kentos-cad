"""Kat yüksekliği: Planlı Alanlar İmar Yönetmeliği 28/1-c, Sivas İmar Yönetmeliği 28/1-c ve parselin plan notları
(docs/adr/8083, 8084).

Kat yükseklikleri, uygulama imar planında daha fazla belirlenmemişse, döşeme üst kotundan döşeme üst kotuna en fazla:

- **1.10.2017** (RG 3.7.2017/30113): konut bölgelerinde zemin ve normal katlarda 3,60 m.
- **12.8.2023** (RG 32277, 2 nci madde ek ibare): aynı; bodrum hariç 3 katı geçmeyen binalarda 4,00 m. Değişiklik yayımı
  günü yürürlüğe girdi, 28/1-c için geçiş hükmü yok: başvuru gününde yürürlükteki hâl okunur.
- **Sivas 28/1-c (10.11.2022):** konut bölgelerinde zemin ve normal katlarda 3,60 m; 4,00 m'si yok.
- **Plan notu** (olguların ``planNotes``'u): 28/1 kat yüksekliklerini plana bırakır, yerel yönetmelik plan aksini
  demiyorsa uygulanır (69/3). Zemin kat yüksekliği notu ±0,00'daki (en alttaki zemin ya da normal) katın, kat
  yüksekliği notu öbür zemin ve normal katların (zemin katın kendi notu yoksa onun da) sınırıdır; notun bağladığı katta
  28/1-c ve Sivas 28/1-c kapsam dışıdır, değerleri farklıysa iletide yanında yazılır. Plan notu bölgenin türünden
  bağımsızdır: konut projesi olmayan binada da okunur.

**Okuma** (olgular ``building.storeys``: kotu, kattan kata yüksekliği, yatak odaları; ``units``, ``dwellings``):

- Bölge türü projede henüz yazılmıyor: kural konut projesinde konut bölgesi sayar ve iletide söyler. Konut projesi bina
  programında konut nitelikli (ya da niteliği yazılmamış) bağımsız bölümü olan projedir; programsız çizimde yatak odası.
- Zemin ve normal katlar ±0,00 ve üstündeki katlardır; bodrum okunmaz, katları sayılmaz. Yükseklik katın kendi
  yüksekliği, yoksa üstündeki katın kotuna kadardır; ikisi de yoksa eksik veri.
- 28/1-d'nin geleneksel kesme taş yığma binası (4,50 m) çizimde okunmaz.
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
PLANA_ONARIM = "Kat yüksekliğini plan notunun değerine düşürün (Proje ayarları › Mimari › Katlar)."

#: Plan notlarının bu kuralın okuduğu ölçüleri (olguların ``planNotes[].key``'i) ve satırdaki adları.
ZEMIN, KAT = "groundStoreyHeight", "storeyHeight"
NOT_SATIRI = {ZEMIN: "Plan notu: zemin kat", KAT: "Plan notu: kat yüksekliği"}
NOT_BASLIGI = {ZEMIN: "Zemin kat yüksekliği (en çok)", KAT: "Kat yüksekliği (en çok)"}


def konut(bina: dict[str, Any]) -> bool:
    """Proje konutun mu: programında konut nitelikli ya da niteliği yazılmamış bağımsız bölüm; programsızsa yatak
    odası."""
    if int(bina.get("units") or 0) > 0:
        return int(bina.get("dwellings") or 0) > 0
    return any(int(s.get("bedrooms") or 0) > 0 for s in bina.get("storeys") or [])


def ustteki(bina: dict[str, Any]) -> list[dict[str, Any]]:
    """Zemin ve normal katlar: ±0,00 ve üstündekiler."""
    return [s for s in bina.get("storeys") or [] if s["elevation"] >= -1e-6]


def plan_notu(bina: dict[str, Any], kat: dict[str, Any], zemin: float | None) -> dict[str, Any] | None:
    """Katı bağlayan plan notu: zemin katın kendi notu, yoksa kat yüksekliği notu; ``zemin`` zemin katın kotu."""
    notlar = {n["key"]: n for n in bina.get("planNotes") or []}
    if zemin is not None and abs(kat["elevation"] - zemin) < 1e-6 and ZEMIN in notlar:
        return notlar[ZEMIN]
    return notlar.get(KAT)


def sinir(hal: y.Hal, katlar: int) -> katman.Sinir:
    """Hâlin ``katlar`` katlı binaya koyduğu en çok yükseklik."""
    a = hal.anahtar
    az = a.get("lowStoreys")
    deger = a["lowMost"] if az is not None and katlar <= int(az) else a["most"]
    return katman.Sinir(float(deger), hal)


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _not_satiri(not_: dict[str, Any], durum: str) -> dict[str, Any]:
    return {"rule": NOT_SATIRI[not_["key"]], "title": NOT_BASLIGI[not_["key"]], "version": None,
            "gazette": not_["source"], "state": durum, "note": ""}


def _kat(kat: dict[str, Any], katlar: int, sivas: bool, gun: date, konut_mu: bool,
         not_: dict[str, Any] | None) -> y.Sonuc:
    """Bir katın ``gun``deki bulgusu ve maddelerin bu kattaki durumu (satırların notu ``kural``'ın). ``not_`` katı
    bağlayan plan notu; konut projesi değilse yalnız o okunur."""
    hu = y.hal("PAİY", "28/1-c", gun)
    hy = y.hal("Sivas", "28/1-c", gun) if sivas else None
    if hu is None:
        return y.Sonuc([], [])
    hallar = [h_ for h_ in (hu, hy) if h_ is not None]
    uids = list(kat.get("rooms") or [])
    h = kat.get("height")
    if h is None:
        kim = katman.PlanNotu(float(not_["value"]), not_["source"]).kural() if not_ else hu.kural()
        ad = f"Plan notu ({not_['source']})" if not_ else hu.kisa()
        bulgu = {"code": "paiy_storey_height_over",
                 "message": f"“{kat['name']}” katının yüksekliği yazılmamış ve üstünde kat yok: kat yüksekliği "
                            f"okunamıyor — {ad}",
                 "repair": "Katın yüksekliğini yazın (Proje ayarları › Mimari › Katlar).", "uids": uids,
                 "rule": kim, "_iz": f"{kat['level']}:yükseklik", "state": y.EKSIK_VERI}
        if not_:
            satirlar = [_not_satiri(not_, y.EKSIK_VERI)] + [_satir(h_, y.KAPSAM_DISI, "") for h_ in hallar]
        else:
            satirlar = [_satir(h_, y.EKSIK_VERI, "") for h_ in hallar]
        return y.Sonuc([bulgu], satirlar)
    ulusal = sinir(hu, katlar) if konut_mu else None
    yerel = sinir(hy, katlar) if hy is not None and konut_mu else None
    if not_:
        plan = katman.PlanNotu(float(not_["value"]), not_["source"])
        b = katman.bulgu("paiy_storey_height_over", f"“{kat['name']}” kat yüksekliği", float(h), False, ulusal, yerel,
                         PLANA_ONARIM, uids, kat["level"], plan=plan)
        durum = y.GECTI if katman.tutar(float(h), False, plan) else y.KALDI
        satirlar = [_not_satiri(not_, durum)] + [_satir(h_, y.KAPSAM_DISI, "") for h_ in hallar]
        return y.Sonuc([b] if b else [], satirlar)
    assert ulusal is not None
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
    """Projenin kat yüksekliği bulguları: her kat başvuru gününün hâlleriyle ve kendi plan notuyla; gün yoksa her kat
    her hâlle denenmiş (``yururluk.parcalarla``)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    sivas = bina.get("local") == "sivas"
    maddeler = ULUSAL + (YEREL if sivas else [])
    katlar = ustteki(bina)
    n = len(katlar)
    konut_mu = konut(bina)
    zemin = min((k["elevation"] for k in katlar), default=None)
    notlu = {k["level"]: plan_notu(bina, k, zemin) for k in katlar}
    okunan = [k for k in katlar if konut_mu or notlu[k["level"]]]
    if not okunan:
        # Satırların hâlleri: başvuru gününün, gün yoksa en yenisi.
        hallar = [y.hal(yon, m, gun) if gun is not None else y.haller(yon, m)[-1] for yon, m in maddeler]
        neden_ = "konut projesi değil (bina programında konut nitelikli bölüm yok)" if katlar else "±0,00 üstünde kat yok"
        return y.Sonuc([], [_satir(h, y.KAPSAM_DISI, neden_) for h in hallar if h is not None])
    if gun is not None and y.hal("PAİY", "28/1-c", gun) is None:
        return _bilinmiyor(gun, [u for k in okunan for u in k.get("rooms") or []][:20])
    sonuclar = y.parcalarla(okunan, lambda k, g: _kat(k, n, sivas, g, konut_mu, notlu[k["level"]]), maddeler, gun,
                            neden, KONU, lambda k: list(k.get("rooms") or []))
    bulgular = [b for s in sonuclar for b in s.bulgular]
    notlar = {n_["key"]: n_ for n_ in bina.get("planNotes") or []}
    satirlar = []
    for r in y.birlesik(sonuclar):
        sayi = r.pop("sayilar")
        kalan, eksik, gecen = sayi.get(y.KALDI, 0), sayi.get(y.EKSIK_VERI, 0), sayi.get(y.GECTI, 0)
        not_ = next((x for k, x in notlar.items() if NOT_SATIRI.get(k) == r["rule"]), None)
        if not_ is not None:
            deger = katman.metre(float(not_["value"]))
            if r["state"] == y.EKSIK_VERI:
                r["note"] = f"{eksik} katın yüksekliği okunamıyor ya da sonucu başvuru gününe bağlı"
            elif r["state"] == y.KALDI:
                r["note"] = f"{kalan} kat plan notunun {deger}'sini aşıyor ({not_['source']})"
            else:
                r["note"] = f"{gecen} kat plan notunun {deger}'si içinde ({not_['source']})"
        elif r["state"] == y.EKSIK_VERI:
            r["note"] = f"{eksik} katın sonucu başvuru gününe ya da yazılmamış yüksekliğe bağlı"
        elif r["state"] == y.KALDI:
            r["note"] = f"{kalan} kat sınırı aşıyor (konut bölgesi, {n} kat)"
        elif r["state"] == y.KAPSAM_DISI:
            r["note"] = ("kat yükseklikleri parselin plan notunda" if konut_mu
                         else "konut projesi değil (bina programında konut nitelikli bölüm yok)")
        else:
            r["note"] = f"{gecen} katın yüksekliği sınırın içinde (konut bölgesi)"
        satirlar.append(r)
    return y.Sonuc(bulgular, satirlar)
