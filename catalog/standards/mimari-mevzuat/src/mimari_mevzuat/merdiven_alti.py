"""Merdiven altı: Planlı Alanlar İmar Yönetmeliği 31/6 (docs/adr/8080).

"Yüksekliği 1.80 metreden az olan merdiven altları kapatılır." KentOS merdivenin gövdesini kesitin çizdiği gibi okur
(basamaklar, alt yüzü döşemesinin taşıyıcı kalınlığıyla, sahanlıklar) ve altında, başladığı döşemeden 1,80 m'den alçak
kalan yerleri ölçer; boşluk olanları (alt yüzü döşemenin üstünde) hangi mahallerin içinde kaldığıyla verir. Bu yerler
bağımsız bölüm net alanından KentOS'un alan hesabında düşülür (4/1-ğ-3); bu kural kapatılıp kapatılmadıklarına bakar:

- **Kaldı:** alçak boşluk adı dolap, depo, kiler, ardiye, vestiyer ya da merdiven altı olmayan bir mahalin içinde
  (salon, hol, merdiven evi …): o mahale açık.
- **Geçti:** alçak boşlukların hepsi böyle kapalı bir mahalde ya da hiçbir mahalin içinde değil (duvarla ayrılmış).
- **Eksik veri:** merdivenin vardığı ve başladığı yerde döşeme yok: alt yüzü, dolayısıyla altında boşluk olup olmadığı
  okunmuyor (KentOS gövdeyi döşemesine kadar dolu sayar).

**Okunan olgular** (``facts.building.underStairs``): her merdivenin altındaki 1,80 m'den alçak yerlerin alanı, boşluk
olanlarınki, döşemesinin bilinip bilinmediği ve boşluğun içinde kaldığı mahaller (kimlik, ad, alan).
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import yururluk as y

MADDELER = [("PAİY", "31/6")]

#: Kapalı merdiven altı sayılan mahallerin adlarındaki sözcükler: bir dolap, bir depo.
_KAPALI = ("DOLAP", "DEPO", "KILER", "ARDIYE", "VESTIYER")

#: Bundan küçük bir parça bir mahale açık sayılmaz (çizimin sınır gürültüsü), m².
_EN_AZ = 0.01


def _katla(metin: str) -> str:
    """Büyük harf, Türkçe harfler katlanmış (İ, ı → I; Ş → S …)."""
    tablo = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                           "ş": "S", "Ş": "S", "ü": "U", "Ü": "U", "â": "A", "Â": "A", "î": "I", "Î": "I"})
    return metin.translate(tablo).upper()


def kapali(ad: str) -> bool:
    """Bir mahal adı kapalı bir merdiven altı mı: merdiven altı, dolap, depo, kiler, ardiye, vestiyer."""
    sozcukler = [w for w in "".join(c if c.isalnum() else " " for c in _katla(ad or "")).split() if w]
    if any(w.startswith(k) for w in sozcukler for k in _KAPALI):
        return True
    return any(a.startswith("MERDIVEN") and b.startswith("ALTI") for a, b in zip(sozcukler, sozcukler[1:]))


def _m2(x: float) -> str:
    return f"{x:.2f}"


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin merdiven altı bulguları."""
    hal = y.hal("PAİY", "31/6", gun)
    merdivenler = [m for m in bina.get("underStairs") or [] if float(m.get("low") or 0) > 0]
    if hal is None:
        if not merdivenler:
            return y.Sonuc([], [])
        ilk = y.haller("PAİY", "31/6")[0]
        ileti = (f"Başvuru günü ({y.yaz(gun)}) PAİY 31/6 maddesinin kural setindeki ilk hâlinden ({ilk.etiket}) önce: "
                 f"o günün metni KentOS'ta yok")
        bulgu = {"code": "regulation_text_unknown", "message": ileti, "repair": "Merdiven altını maddenin başvuru "
                 "günündeki metnine göre uzman incelesin.", "uids": sorted(m["stair"] for m in merdivenler),
                 "rule": "PAİY 31/6", "state": y.UZMAN}
        return y.Sonuc([bulgu], [{"rule": "PAİY 31/6", "title": ilk.baslik, "version": None, "gazette": None,
                                  "state": y.UZMAN, "note": ileti}])
    if not merdivenler:
        return y.Sonuc([], [_satir(hal, y.KAPSAM_DISI, "altında 1,80 m'den alçak yer olan merdiven yok")])
    sinir = float(hal.anahtar.get("clear", 1.80))
    bulgular: list[dict[str, Any]] = []
    eksik = 0
    for m in merdivenler:
        if not m.get("waist"):
            eksik += 1
            continue
        for r in m.get("rooms") or []:
            alan = float(r.get("area") or 0)
            if alan < _EN_AZ or kapali(r.get("name") or ""):
                continue
            ad = r.get("name") or "Mahal"
            bulgular.append({
                "code": "paiy_under_stair_open",
                "message": f"Merdivenin altında {sinir:.2f} m'den alçak {_m2(alan)} m² boşluk “{ad}” mahaline açık; "
                           f"31/6 kapatılmasını ister — {hal.kisa()}",
                "repair": "Merdiven altını yüksekliğin 1,80 m olduğu yerden duvar ya da bölmeyle kapatın, ya da orada "
                          "bir dolap ya da depo mahali çizin.",
                "uids": [m["stair"], r["uid"]],
                "rule": hal.kural(),
                "_iz": f"{m['stair']}:{r['uid']}",
            })
    acik = len({b["uids"][0] for b in bulgular})
    if bulgular:
        satir = _satir(hal, y.KALDI, f"{acik} merdivenin altı açık")
    elif eksik:
        satir = _satir(hal, y.EKSIK_VERI, f"{eksik} merdivenin döşemesi yok: alt yüzü okunmuyor")
    else:
        satir = _satir(hal, y.GECTI, f"{len(merdivenler)} merdivenin 1,80 m'den alçak altı kapalı")
    return y.Sonuc(bulgular, [satir])


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin merdiven altı bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "merdiven altı kuralı")
