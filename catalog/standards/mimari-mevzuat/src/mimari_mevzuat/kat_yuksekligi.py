"""Kat yüksekliği: Planlı Alanlar İmar Yönetmeliği 28/1, Sivas İmar Yönetmeliği 28/1 ve parselin plan notları
(docs/adr/8083, 8084, 8085).

Kat yükseklikleri, uygulama imar planında daha fazla belirlenmemişse, döşeme üst kotundan döşeme üst kotuna en fazla,
parselin plandaki kullanımına (bölgesine) göre:

- **a) Ticaret alanı:** zemin katta 4,50 m, asma katlı zemin katta 5,50 m, öbür katlarda 4,00 m (Sivas 5,00; 6,50; 4,00).
- **b) Karma alan** (ticaretin de yapılabildiği): zemin 4,50, asma katlı zemin 5,50; öbür katlar konutsa 3,60, değilse
  4,00 (Sivas zemin 5,00, asma katlı 6,50).
- **c) Konut alanı:** zemin ve normal katlarda 3,60 m; 12.8.2023'ten (RG 32277) bodrum hariç 3 katı geçmeyen binalarda
  4,00 m (Sivas 3,60).
- **ç) Zemin katında ticaret yapılabilen konut alanı:** zemin 4,50, asma katlı zemin 5,5 (12.8.2023'te kaldırıldı:
  sonra asma katlı zemin kat da 4,50), öbür katlar 3,60 (Sivas 4,50; 5,50; 3,60).
- **d) Geleneksel kesme taş yığma bina** (9.10.2021'den, RG 31623; Sivas 28/1-d): konut bölgelerinde, geleneksel mimari
  öğeler barındıran ve taşıyıcı sistemi kesme taş yığma bina olarak tasarlanan zemin dahil 3 katı geçmeyen binalarda
  zemin ve normal katlarda 4,50 m. Beyanı projenindir (``structuralSystem``: ``cutStoneMasonry``, ``traditional``;
  docs/adr/8094); konut alanında da zemin katında ticaret yapılabilen konut alanında da (ikisi de konut bölgesi) c ya da
  ç'nin yerine okunur; 9.10.2021'den önceki başvuruda c ya da ç.
- **Öbür kullanım** (eğitim, sağlık, sanayi …): 28/1 kat yüksekliği vermez.
- **Plan notu** (``planNotes``): 28/1 kat yüksekliklerini plana bırakır, yerel yönetmelik plan aksini demiyorsa
  uygulanır (69/3). Zemin kat yüksekliği notu zemin katın, kat yüksekliği notu öbür zemin ve normal katların (zemin
  katın kendi notu yoksa onun da) sınırıdır; notun bağladığı katta maddeler kapsam dışıdır, değerleri farklıysa iletide
  yanında yazılır. Plan notu bölgeden bağımsız okunur.

Değişiklikler yayımı günü yürürlüğe girdi, geçiş hükmü 28/1'i saymaz: başvuru gününde yürürlükteki hâl okunur.

**Okuma** (olgular ``building.storeys``: kotu, kattan kata yüksekliği, yatak odaları, konut mahalleri, asma kat olup
olmadığı; ``zone``; ``units``, ``dwellings``):

- Bölge projenin yazdığıdır. Yazılmamışsa konut projesinde konut alanı sayılır (iletide “konut bölgesi”); konut projesi
  olmayan binada sınır planın kullanımına bağlıdır: eksik veri. Konut projesi bina programında konut nitelikli (ya da
  niteliği yazılmamış) bağımsız bölümü olan projedir; programsız çizimde yatak odası.
- Zemin ve normal katlar ±0,00 ve üstündeki katlardır; bodrum okunmaz, katları sayılmaz. Zemin kat bunların en altıdır.
- Asma kat (adı Asma kat olan kat, PAİY 4/1-d) zemin katın parçasıdır: üstünde asma katı olan zemin kat asma katlı zemin
  kattır, yüksekliği asma katın üstündeki ilk kata kadardır; asma kat ayrı kat sayılmaz.
- Yükseklik katın kendi yüksekliği, yoksa üstündeki katın kotuna kadardır; ikisi de yoksa eksik veri.
- Karma alanda konut mahali (bağımsız bölümü konut olan mahal) taşıyan kat konut katıdır.
- 28/1-d'nin binası projenin taşıyıcı sistem beyanıyla (kesme taş yığma, geleneksel mimari öğeli) ve zemin dahil kat
  sayısıyla (zemin ve normal katlar, en çok 3) okunur.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import katman
from . import yururluk as y

KONU = "kat yüksekliği kuralı"
ONARIM = "Kat yüksekliğini düşürün (Proje ayarları › Mimari › Katlar) ya da planın kendi notuna bakın."
PLANA_ONARIM = "Kat yüksekliğini plan notunun değerine düşürün (Proje ayarları › Mimari › Katlar)."
BOLGE_ONARIM = "Proje ayarları › Mimari › İmar durumu'nda Bölge'yi (parselin planda ayrıldığı kullanımı) seçin."

#: Bölgeler (olguların ``zone``'u) ve okudukları bent.
BOLGE_MADDESI = {"residential": "28/1-c", "residentialShops": "28/1-ç", "mixed": "28/1-b", "commercial": "28/1-a"}

#: Geleneksel kesme taş yığma binanın bendi ve onu okuyan bölgelerin (konut bölgeleri) bentleri.
GELENEKSEL, KONUT_BENTLERI = "28/1-d", ("28/1-c", "28/1-ç")

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


def geleneksel(bina: dict[str, Any]) -> bool:
    """Proje binayı geleneksel mimari öğeli kesme taş yığma bina diye beyan ediyor mu (docs/adr/8094)."""
    return bina.get("structuralSystem") == "cutStoneMasonry" and bool(bina.get("traditional"))


def ustteki(bina: dict[str, Any]) -> list[dict[str, Any]]:
    """Zemin ve normal katlar: ±0,00 ve üstündeki, asma kat olmayan katlar."""
    return [s for s in bina.get("storeys") or [] if s["elevation"] >= -1e-6 and not s.get("mezzanine")]


def zemin_yuksekligi(bina: dict[str, Any], zemin: dict[str, Any]) -> tuple[float | None, list[dict[str, Any]]]:
    """Zemin katın yüksekliği ve üstündeki asma katlar: asma katı varsa asma katın üstündeki ilk kata kadar (o yoksa
    kendisinin ve asma katlarının yükseklikleri toplamı); yoksa kendi yüksekliği."""
    ust = sorted((s for s in bina.get("storeys") or [] if s["elevation"] > zemin["elevation"] + 1e-6),
                 key=lambda s: s["elevation"])
    asma: list[dict[str, Any]] = []
    sonraki = None
    for s in ust:
        if not s.get("mezzanine"):
            sonraki = s
            break
        asma.append(s)
    if not asma:
        return zemin.get("height"), []
    if sonraki is not None:
        return round(sonraki["elevation"] - zemin["elevation"], 3), asma
    if zemin.get("height") is None or any(m.get("height") is None for m in asma):
        return None, asma
    return round(zemin["height"] + sum(m["height"] for m in asma), 3), asma


def plan_notu(bina: dict[str, Any], zemin_mi: bool) -> dict[str, Any] | None:
    """Katı bağlayan plan notu: zemin katın kendi notu, yoksa kat yüksekliği notu."""
    notlar = {n["key"]: n for n in bina.get("planNotes") or []}
    if zemin_mi and ZEMIN in notlar:
        return notlar[ZEMIN]
    return notlar.get(KAT)


def sinir(hal: y.Hal, katlar: int, zemin_mi: bool = False, asma: bool = False,
          konut_kati: bool = True) -> katman.Sinir:
    """Hâlin bir kata koyduğu en çok yükseklik: c bendinde binanın kat sayısına göre, öbürlerinde zemin kata (asma katlı
    olup olmadığına), karma alanda katın konut olup olmadığına göre."""
    a = hal.anahtar
    if "most" in a:
        az = a.get("lowStoreys")
        deger = a["lowMost"] if az is not None and katlar <= int(az) else a["most"]
    elif zemin_mi:
        deger = a["groundMezzanine"] if asma and a.get("groundMezzanine") is not None else a["ground"]
    elif "otherDwelling" in a:
        deger = a["otherDwelling"] if konut_kati else a["otherNonDwelling"]
    else:
        deger = a["other"]
    return katman.Sinir(float(deger), hal)


def nitelik(madde: str, katlar: int, zemin_mi: bool, asma: bool, konut_kati: bool) -> str:
    """Ölçünün ve sınırın ardındaki ayraç: bölge ve katın ne olduğu."""
    if madde == "28/1-c":
        return f" (konut bölgesi, {katlar} kat" + (", asma katlı zemin kat)" if zemin_mi and asma else ")")
    if madde == GELENEKSEL:
        return f" (geleneksel kesme taş yığma bina, konut bölgesi, zemin dahil {katlar} kat)"
    yer = "asma katlı zemin kat" if zemin_mi and asma else "zemin kat" if zemin_mi else "normal kat"
    if madde == "28/1-b" and not zemin_mi:
        yer = "konut katı" if konut_kati else "konut dışı kat"
    bolge = {"28/1-a": "ticaret alanı", "28/1-b": "karma alan",
             "28/1-ç": "zemin katında ticaret yapılabilen konut alanı"}[madde]
    return f" ({bolge}, {yer})"


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _not_satiri(not_: dict[str, Any], durum: str) -> dict[str, Any]:
    return {"rule": NOT_SATIRI[not_["key"]], "title": NOT_BASLIGI[not_["key"]], "version": None,
            "gazette": not_["source"], "state": durum, "note": ""}


def _bolge_satiri(durum: str, not_: str) -> dict[str, Any]:
    return {"rule": "PAİY 28/1", "title": "Kat yüksekliği (bölge)", "version": None, "gazette": None, "state": durum,
            "note": not_}


def _kat(p: dict[str, Any], katlar: int, madde: str | None, sivas: bool, gun: date) -> y.Sonuc:
    """Bir katın ``gun``deki bulgusu ve maddelerin bu kattaki durumu (satırların notu ``kural``'ın). ``madde`` bölgenin
    bendi; yoksa yalnız katın plan notu okunur."""
    kat, not_ = p["kat"], p["not"]
    hu = y.hal("PAİY", madde, gun) if madde else None
    hy = y.hal("Sivas", madde, gun) if madde and sivas else None
    if madde and hu is None:
        return y.Sonuc([], [])
    hallar = [h_ for h_ in (hu, hy) if h_ is not None]
    uids, h = p["uids"], p["h"]
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
    konut_kati = int(kat.get("dwellingRooms") or 0) > 0
    ulusal = sinir(hu, katlar, p["zemin"], p["asma"], konut_kati) if hu is not None else None
    yerel = sinir(hy, katlar, p["zemin"], p["asma"], konut_kati) if hy is not None else None
    ozne = f"“{kat['name']}” kat yüksekliği"
    if not_:
        plan = katman.PlanNotu(float(not_["value"]), not_["source"])
        b = katman.bulgu("paiy_storey_height_over", ozne, float(h), False, ulusal, yerel, PLANA_ONARIM, uids,
                         kat["level"], plan=plan)
        durum = y.GECTI if katman.tutar(float(h), False, plan) else y.KALDI
        satirlar = [_not_satiri(not_, durum)] + [_satir(h_, y.KAPSAM_DISI, "") for h_ in hallar]
        return y.Sonuc([b] if b else [], satirlar)
    assert ulusal is not None and madde is not None
    b = katman.bulgu("paiy_storey_height_over", ozne, float(h), False, ulusal, yerel, ONARIM, uids, kat["level"],
                     nitelik(madde, katlar, p["zemin"], p["asma"], konut_kati))
    du, dy = katman.durumlar(float(h), False, ulusal, yerel)
    satirlar = [_satir(hu, du, "")] + ([_satir(hy, dy, "")] if hy is not None and dy is not None else [])
    return y.Sonuc([b] if b else [], satirlar)


def _bilinmiyor(madde: str, gun: date, uids: list[str]) -> y.Sonuc:
    ilk = y.haller("PAİY", madde)[0]
    ileti = (f"Başvuru günü ({y.yaz(gun)}) PAİY {madde}'in kural setindeki ilk hâlinden ({ilk.etiket}) önce: o günün "
             f"metni KentOS'ta yok")
    bulgu = {"code": "regulation_text_unknown", "message": ileti,
             "repair": "Kat yüksekliklerini maddenin başvuru günündeki metnine göre uzman incelesin.", "uids": uids,
             "rule": f"PAİY {madde}", "_iz": f"{madde}:bilinmiyor", "state": y.UZMAN}
    satir = {"rule": f"PAİY {madde}", "title": ilk.baslik, "version": None, "gazette": None, "state": y.UZMAN,
             "note": ileti}
    return y.Sonuc([bulgu], [satir])


def parcalar(bina: dict[str, Any], madde: str | None) -> list[dict[str, Any]]:
    """Okunan katlar: zemin ve normal katlar (zemin kat asma katlarıyla), yükseklikleri, nesneleri ve plan notları;
    bölgenin bendi yoksa yalnız plan notu olanlar."""
    katlar = ustteki(bina)
    zemin = min(katlar, key=lambda s: s["elevation"]) if katlar else None
    out = []
    for k in katlar:
        zemin_mi = k is zemin
        if zemin_mi:
            h, asma = zemin_yuksekligi(bina, k)
        else:
            h, asma = k.get("height"), []
        not_ = plan_notu(bina, zemin_mi)
        if madde is None and not_ is None:
            continue
        uids = list(k.get("rooms") or []) + [u for m in asma for u in m.get("rooms") or []]
        out.append({"kat": k, "h": h, "zemin": zemin_mi, "asma": bool(asma), "uids": uids, "not": not_})
    return out


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin kat yüksekliği bulguları: her kat bölgenin bendinin başvuru günündeki hâliyle ve kendi plan notuyla; gün
    yoksa her kat her hâlle denenmiş (``yururluk.parcalarla``)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    sivas = bina.get("local") == "sivas"
    zone = bina.get("zone")
    konut_mu = konut(bina)
    madde = BOLGE_MADDESI.get(zone) if zone else ("28/1-c" if konut_mu else None)
    katlar = ustteki(bina)
    n = len(katlar)
    okunan = parcalar(bina, madde)
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    # Bölgenin bendi olmayan bina: öbür kullanım kapsam dışı, yazılmamış bölge (konut projesi değil) eksik veri.
    if madde is None:
        notsuz = [k for k in katlar if not any(p["kat"] is k for p in okunan)]
        if zone == "other":
            satirlar.append(_bolge_satiri(y.KAPSAM_DISI, "planın kullanımı 28/1'in saymadığı bir kullanım"))
        elif notsuz:
            bulgular.append({
                "code": "paiy_storey_height_over",
                "message": "Bölge yazılmamış: konut projesi olmayan binada kat yüksekliğinin sınırı parselin planda "
                           "ayrıldığı kullanıma bağlı (ticaret alanı 28/1-a, karma alan 28/1-b) — PAİY 28/1",
                "repair": BOLGE_ONARIM, "uids": [u for k in notsuz for u in k.get("rooms") or []][:20],
                "rule": "PAİY 28/1", "_iz": "28/1:bölge", "state": y.EKSIK_VERI})
            satirlar.append(_bolge_satiri(y.EKSIK_VERI, "bölge yazılmamış ve proje konut projesi değil"))
    # 28/1-d: konut bölgesindeki, zemin dahil 3 katı geçmeyen geleneksel kesme taş yığma bina; bendin eklendiği
    # 9.10.2021'den önceki başvuruda bölgenin kendi bendi (docs/adr/8094).
    en_cok = int(y.haller("PAİY", GELENEKSEL)[-1].anahtar["storeys"])
    d_okunur = madde in KONUT_BENTLERI and geleneksel(bina) and 0 < n <= en_cok

    def bent(g: date) -> str | None:
        h = y.hal("PAİY", GELENEKSEL, g) if d_okunur else None
        return GELENEKSEL if h is not None and not h.anahtar.get("absent") else madde

    ekler = [GELENEKSEL] if d_okunur else []
    maddeler = ([("PAİY", m) for m in [madde or "28/1-c", *ekler]]
                + ([("Sivas", m) for m in [madde or "28/1-c", *ekler]] if sivas else []))
    if madde and not okunan:
        hallar = [y.hal(yon, m, gun) if gun is not None else y.haller(yon, m)[-1] for yon, m in maddeler]
        satirlar += [_satir(h, y.KAPSAM_DISI, "±0,00 üstünde kat yok") for h in hallar if h is not None]
        return y.Sonuc(bulgular, satirlar)
    if madde and gun is not None and y.hal("PAİY", madde, gun) is None:
        sonuc = _bilinmiyor(madde, gun, [u for p in okunan for u in p["uids"]][:20])
        return y.Sonuc(bulgular + sonuc.bulgular, satirlar + sonuc.satirlar)
    sonuclar = y.parcalarla(okunan, lambda p, g: _kat(p, n, bent(g), sivas, g), maddeler, gun, neden, KONU,
                            lambda p: list(p["uids"]))
    bulgular += [b for s in sonuclar for b in s.bulgular]
    notlar = {n_["key"]: n_ for n_ in bina.get("planNotes") or []}
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
            r["note"] = f"{kalan} kat sınırı aşıyor"
        elif r["state"] == y.KAPSAM_DISI:
            r["note"] = "kat yükseklikleri parselin plan notunda"
        else:
            r["note"] = f"{gecen} katın yüksekliği sınırın içinde"
        if zone is None and madde == "28/1-c" and not r["rule"].startswith("Plan notu"):
            r["note"] += " (bölge yazılmamış: konut projesi, konut alanı sayıldı)"
        satirlar.append(r)
    return y.Sonuc(bulgular, satirlar)
