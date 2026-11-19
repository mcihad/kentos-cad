"""Bina yüksekliği ve kat adedi: Planlı Alanlar İmar Yönetmeliği 5/11 ve parselin plan notu (docs/adr/8098).

- **5/11:** yeni yapılacak binalarda uygulama imar planında (planda olmaması hâlinde yönetmelikte) gösterilen kat adedi ya
  da bina yüksekliği aşılamaz. Planın değerleri projenin plan notlarıdır (``planNotes``: ``buildingHeight``, ``storeys``);
  KentOS planın notunu okumaz, değeri planı okuyan yazar. Notu olmayan ölçünün satırı yazılmaz.
- **Bina yüksekliği** (4/1-n): binanın kot aldığı noktadan saçak seviyesine; saçak seviyesi son kat tavan döşemesinin
  üst kotu (4/1-ppp). Binanın ±0,00'ı PAİY'nin ±0,00 kotudur, binanın kot aldığı nokta (zemin katın tabanı ondan en çok
  1,20 m yukarıda, 4/1-yyy); bina programının binası kendi ±0,00'ını söylüyorsa o (docs/adr/8154): yükseklik ±0,00 ve
  üstündeki en üst katın (asma kat ve çatı arası dışında) kotu artı kattan kata yüksekliğidir, en yükseği. En üst katın
  yüksekliği okunmuyorsa eksik veri.
- **Kat adedi** (9/1-c: binanın kot aldığı noktaya göre): ±0,00 ve üstündeki katlar, zemin ve normal katlar; asma kat
  zemin katın parçasıdır (4/1-d), adında Çatı geçen kat çatı arasıdır, bodrum zemin katın altındadır: sayılmaz. Kat
  adedi binanındır: birden çok binada en çok katlısı okunur (docs/adr/8154).

Plan notu başvuru gününe bağlı değildir (docs/adr/8084 K91): satırın hâli yok, kaynağı notun yeridir.

**Okunan olgular** (``facts.building``): katlar (kot, kattan kata yükseklik, asma kat olup olmadığı, adı, mahalleri) ve
plan notları.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import kat_yuksekligi
from . import katman
from . import yururluk as y

YUKSEKLIK, KAT = "buildingHeight", "storeys"
SATIR = {YUKSEKLIK: "Plan notu: bina yüksekliği", KAT: "Plan notu: kat adedi"}
BASLIK = {YUKSEKLIK: "Bina yüksekliği (en çok)", KAT: "Kat adedi (en çok)"}


def _katla(metin: str) -> str:
    tablo = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                           "ş": "S", "Ş": "S", "ü": "U", "Ü": "U"})
    return metin.translate(tablo).upper()


def katlar(bina: dict[str, Any]) -> list[dict[str, Any]]:
    """Kat adedine sayılan katlar: binasının ±0,00'ı ve üstündeki, asma kat ve çatı arası olmayan katlar, binalarının
    ±0,00'ından aşağıdan yukarı (docs/adr/8154)."""
    return sorted((k for k in kat_yuksekligi.ustteki(bina) if "CATI" not in _katla(k.get("name") or "")),
                  key=katman.kot)


def _satir(not_: dict[str, Any], durum: str, ne: str) -> dict[str, Any]:
    return {"rule": SATIR[not_["key"]], "title": BASLIK[not_["key"]], "version": None, "gazette": not_["source"],
            "state": durum, "note": ne}


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin bina yüksekliği ve kat adedi bulguları (plan notu günden bağımsız)."""
    del gun, neden
    bina = facts.get("building") or {}
    notlar = {n["key"]: n for n in bina.get("planNotes") or []}
    if not bina.get("storeys") or not (YUKSEKLIK in notlar or KAT in notlar):
        return y.Sonuc([], [])
    sayilan = katlar(bina)
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    if YUKSEKLIK in notlar:
        n = notlar[YUKSEKLIK]
        plan = katman.PlanNotu(float(n["value"]), n["source"])
        ust = sayilan[-1] if sayilan else None
        if ust is None:
            satirlar.append(_satir(n, y.KAPSAM_DISI, "±0,00 üstünde kat yok"))
        elif ust.get("height") is None:
            bulgular.append({
                "code": "paiy_building_height_over",
                "message": f"“{ust['name']}” katının yüksekliği yazılmamış ve üstünde kat yok: saçak seviyesi, bina "
                           f"yüksekliği okunamıyor — {plan.kisa()}",
                "repair": "En üst katın yüksekliğini yazın (Proje ayarları › Mimari › Katlar).",
                "uids": list(ust.get("rooms") or [])[:20], "rule": plan.kural(), "_iz": "yükseklik:okunmuyor",
                "state": y.EKSIK_VERI})
            satirlar.append(_satir(n, y.EKSIK_VERI, "en üst katın yüksekliği yazılmamış"))
        else:
            h = katman.kot(ust) + float(ust["height"])
            if katman.tutar(h, False, plan):
                satirlar.append(_satir(n, y.GECTI, f"bina yüksekliği {katman.metre(h)} ≤ {katman.metre(plan.deger)}"))
            else:
                bulgular.append({
                    "code": "paiy_building_height_over",
                    "message": f"Bina yüksekliği {katman.metre(h)} > {katman.metre(plan.deger)} (±0,00'dan “{ust['name']}” "
                               f"katının tavan döşemesinin üstüne) — {plan.kisa()}; PAİY 5/11",
                    "repair": "Katları alçaltın ya da kat sayısını azaltın: bina yüksekliği planın notunu aşamaz.",
                    "uids": list(ust.get("rooms") or [])[:20], "rule": plan.kural(), "_iz": f"yükseklik:{plan.deger}"})
                satirlar.append(_satir(n, y.KALDI, f"bina yüksekliği {katman.metre(h)} > {katman.metre(plan.deger)}"))
    if KAT in notlar:
        n = notlar[KAT]
        sinir = int(round(float(n["value"])))
        # Kat adedi binanındır (docs/adr/8154): her binanın katları, en çok katlısı.
        gruplar: dict[Any, list[dict[str, Any]]] = {}
        for k in sayilan:
            gruplar.setdefault(k.get("building"), []).append(k)
        en_cok = max(gruplar.values(), key=len) if gruplar else []
        kac = len(en_cok)
        if kac <= sinir:
            satirlar.append(_satir(n, y.GECTI if kac else y.KAPSAM_DISI, f"{kac} kat ≤ {sinir} kat"))
        else:
            fazla = en_cok[sinir:]
            bulgular.append({
                "code": "paiy_storey_count_over",
                "message": f"Kat adedi {kac} > {sinir} (±0,00 ve üstündeki zemin ve normal katlar) — "
                           f"{katman.PlanNotu(float(n['value']), n['source']).kisa()}; PAİY 5/11",
                "repair": "Kat adedini planın notuna indirin; bodrum, asma kat ve çatı arası sayılmaz.",
                "uids": [u for k in fazla for u in k.get("rooms") or []][:20],
                "rule": katman.PlanNotu(float(n["value"]), n["source"]).kural(), "_iz": f"kat:{sinir}"})
            satirlar.append(_satir(n, y.KALDI, f"{kac} kat > {sinir} kat"))
    return y.Sonuc(bulgular, satirlar)
