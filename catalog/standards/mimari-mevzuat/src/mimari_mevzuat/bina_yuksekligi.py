"""Bina yüksekliği ve kat adedi: Planlı Alanlar İmar Yönetmeliği 5/11 ve parselin plan notu (docs/adr/8098).

- **5/11:** yeni yapılacak binalarda uygulama imar planında (planda olmaması hâlinde yönetmelikte) gösterilen kat adedi ya
  da bina yüksekliği aşılamaz. Planın değerleri projenin plan notlarıdır (``planNotes``: ``buildingHeight``, ``storeys``);
  KentOS planın notunu okumaz, değeri planı okuyan yazar. Notu olmayan ölçünün satırı yazılmaz.
- **Bina yüksekliği** (4/1-n): binanın kot aldığı noktadan saçak seviyesine; saçak seviyesi son kat tavan döşemesinin
  üst kotu (4/1-ppp). Binanın ±0,00'ı PAİY'nin ±0,00 kotudur, binanın kot aldığı nokta (zemin katın tabanı ondan en çok
  1,20 m yukarıda, 4/1-yyy); bina programının binası kendi ±0,00'ını söylüyorsa o (docs/adr/8154): yükseklik ±0,00 ve
  üstündeki en üst katın (asma kat ve çatı arası dışında) kotu artı kattan kata yüksekliğidir, en yükseği. En üst katın
  yüksekliği okunmuyorsa eksik veri.
- **Bodrum kat adedi** (docs/adr/8162): planın notu (``basementsMost``) her binanın zemin katının altındaki katlarını
  (binasının ±0,00'ının altında, asma kat olmayan) sayar: en çok bodrumlu bina okunur.
- **Kat adedi** (9/1-c: binanın kot aldığı noktaya göre): ±0,00 ve üstündeki katlar, zemin ve normal katlar; asma kat
  zemin katın parçasıdır (4/1-d), adında Çatı geçen kat çatı arasıdır, bodrum zemin katın altındadır: sayılmaz. Kat
  adedi binanındır: birden çok binada en çok katlısı okunur (docs/adr/8154).
- **Binanın kendi notları** (docs/adr/8163; kademe hattı, PAİY 4/1-mm, 7/12, 7/14): bir bina kendi notlarını
  söylüyorsa her bina kendi notlarıyla (yazmadığı ölçüde projeninkiyle) ayrı denetlenir, ileti binanın adıyla; binanın
  kendi notunun satırı binanın adıyla ayrıdır, projenin notunun satırı onu okuyan binaların sonucudur (hiçbiri
  okumuyorsa kapsam dışı).

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

YUKSEKLIK, KAT, BODRUM = "buildingHeight", "storeys", "basementsMost"
SATIR = {YUKSEKLIK: "Plan notu: bina yüksekliği", KAT: "Plan notu: kat adedi", BODRUM: "Plan notu: bodrum kat adedi"}
BASLIK = {YUKSEKLIK: "Bina yüksekliği (en çok)", KAT: "Kat adedi (en çok)", BODRUM: "Bodrum kat adedi (en çok)"}


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
    return {"rule": katman.not_satiri(SATIR[not_["key"]], not_), "title": BASLIK[not_["key"]], "version": None,
            "gazette": not_["source"], "state": durum, "note": ne}


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin bina yüksekliği ve kat adedi bulguları (plan notu günden bağımsız); bir bina kendi notlarını
    söylüyorsa bina bina (docs/adr/8163)."""
    del gun, neden
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    if not katman.kendi_notu_var(bina):
        return _bina(bina, bina.get("planNotes") or [], "")
    gruplar = katman.binalara(bina)
    sonuclar = []
    for katlar in gruplar:
        ad = katman.bina_adi(katlar) if len(gruplar) > 1 else None
        sonuclar.append(_bina(dict(bina, storeys=katlar), katman.plan_notlari(bina, katlar), f"{ad}: " if ad else ""))
    satirlar = []
    for r in y.birlesik(sonuclar):
        r.pop("sayilar")
        notlar = [x["note"] for s in sonuclar for x in s.satirlar
                  if x["rule"] == r["rule"] and x["state"] == r["state"]]
        r["note"] = " · ".join(dict.fromkeys(n for n in notlar if n))
        satirlar.append(r)
    # Projenin notu: her binanın aynı ölçüde kendi notu varsa okunmaz.
    okunan = {r["rule"] for r in satirlar}
    for n in bina.get("planNotes") or []:
        if n["key"] in SATIR and SATIR[n["key"]] not in okunan:
            satirlar.append(_satir(n, y.KAPSAM_DISI, "her binanın kendi notu var"))
    return y.Sonuc([b for s in sonuclar for b in s.bulgular], satirlar)


def _bina(bina: dict[str, Any], okunan: list[dict[str, Any]], ad: str) -> y.Sonuc:
    """``bina``nın katlarının bulguları ``okunan`` notlarla; ``ad`` iletinin başı (“A Blok: ”), bina bina okunurken."""
    notlar = {n["key"]: n for n in okunan if n["key"] in SATIR}
    if not notlar:
        return y.Sonuc([], [])
    sayilan = katlar(bina)
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    # Satırın notu: projenin notunu birden çok bina okuyorsa binanın adıyla.
    def ne(n: dict[str, Any], metin: str) -> str:
        return metin if n.get("_bina") else f"{ad}{metin}"

    if YUKSEKLIK in notlar:
        n = notlar[YUKSEKLIK]
        plan = katman.plan_notu(n)
        ust = sayilan[-1] if sayilan else None
        if ust is None:
            satirlar.append(_satir(n, y.KAPSAM_DISI, ne(n, "±0,00 üstünde kat yok")))
        elif ust.get("height") is None:
            bulgular.append({
                "code": "paiy_building_height_over",
                "message": f"{ad}“{ust['name']}” katının yüksekliği yazılmamış ve üstünde kat yok: saçak seviyesi, "
                           f"bina yüksekliği okunamıyor — {plan.kisa()}",
                "repair": "En üst katın yüksekliğini yazın (Proje ayarları › Mimari › Katlar).",
                "uids": list(ust.get("rooms") or [])[:20], "rule": plan.kural(), "_iz": "yükseklik:okunmuyor",
                "state": y.EKSIK_VERI})
            satirlar.append(_satir(n, y.EKSIK_VERI, ne(n, "en üst katın yüksekliği yazılmamış")))
        else:
            h = katman.kot(ust) + float(ust["height"])
            if katman.tutar(h, False, plan):
                satirlar.append(_satir(n, y.GECTI, ne(n, f"bina yüksekliği {katman.metre(h)} ≤ "
                                                         f"{katman.metre(plan.deger)}")))
            else:
                bulgular.append({
                    "code": "paiy_building_height_over",
                    "message": f"{ad}Bina yüksekliği {katman.metre(h)} > {katman.metre(plan.deger)} (±0,00'dan "
                               f"“{ust['name']}” katının tavan döşemesinin üstüne) — {plan.kisa()}; PAİY 5/11",
                    "repair": "Katları alçaltın ya da kat sayısını azaltın: bina yüksekliği planın notunu aşamaz.",
                    "uids": list(ust.get("rooms") or [])[:20], "rule": plan.kural(), "_iz": f"yükseklik:{plan.deger}"})
                satirlar.append(_satir(n, y.KALDI, ne(n, f"bina yüksekliği {katman.metre(h)} > "
                                                         f"{katman.metre(plan.deger)}")))
    if KAT in notlar:
        n = notlar[KAT]
        sinir = int(round(float(n["value"])))
        plan = katman.plan_notu(n)
        # Kat adedi binanındır (docs/adr/8154): her binanın katları, en çok katlısı.
        gruplar: dict[Any, list[dict[str, Any]]] = {}
        for k in sayilan:
            gruplar.setdefault(k.get("building"), []).append(k)
        en_cok = max(gruplar.values(), key=len) if gruplar else []
        kac = len(en_cok)
        if kac <= sinir:
            satirlar.append(_satir(n, y.GECTI if kac else y.KAPSAM_DISI, ne(n, f"{kac} kat ≤ {sinir} kat")))
        else:
            fazla = en_cok[sinir:]
            bulgular.append({
                "code": "paiy_storey_count_over",
                "message": f"{ad}Kat adedi {kac} > {sinir} (±0,00 ve üstündeki zemin ve normal katlar) — "
                           f"{plan.kisa()}; PAİY 5/11",
                "repair": "Kat adedini planın notuna indirin; bodrum, asma kat ve çatı arası sayılmaz.",
                "uids": [u for k in fazla for u in k.get("rooms") or []][:20], "rule": plan.kural(),
                "_iz": f"kat:{sinir}"})
            satirlar.append(_satir(n, y.KALDI, ne(n, f"{kac} kat > {sinir} kat")))
    if BODRUM in notlar:
        n = notlar[BODRUM]
        sinir = int(round(float(n["value"])))
        plan = katman.plan_notu(n)
        # Bina bina zemin katın altındaki katlar (asma kat değil), binasının ±0,00'ından; en çok bodrumlusu.
        gruplar = {}
        for k in bina.get("storeys") or []:
            if katman.kot(k) < -1e-6 and not k.get("mezzanine"):
                gruplar.setdefault(k.get("building"), []).append(k)
        en_cok = max(gruplar.values(), key=len) if gruplar else []
        kac = len(en_cok)
        if kac <= sinir:
            satirlar.append(_satir(n, y.GECTI if kac else y.KAPSAM_DISI, ne(n, f"{kac} bodrum kat ≤ {sinir}")))
        else:
            # Planın saydığından derindekiler: en alttakiler.
            fazla = sorted(en_cok, key=katman.kot)[: kac - sinir]
            bulgular.append({
                "code": "paiy_basement_count_over",
                "message": f"{ad}Bodrum kat adedi {kac} > {sinir} (zemin katın altındaki katlar) — {plan.kisa()}",
                "repair": "Bodrum katlarını planın notuna indirin.",
                "uids": [u for k in fazla for u in k.get("rooms") or []][:20], "rule": plan.kural(),
                "_iz": f"bodrum:{sinir}"})
            satirlar.append(_satir(n, y.KALDI, ne(n, f"{kac} bodrum kat > {sinir}")))
    return y.Sonuc(bulgular, satirlar)
