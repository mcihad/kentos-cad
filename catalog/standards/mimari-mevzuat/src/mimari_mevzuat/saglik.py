"""Sağlık yapılarının yatılan katları ve hastane koridorları: Binaların Yangından Korunması Hakkında Yönetmelik 49/1-b ve
49/2 (docs/adr/8110; plan/bim-plan.md B22, BIM-13'ün konut dışı kullanımları).

- **49/1-b (9.9.2009):** hastanelerin ve bakımevlerinin 300 m²'den büyük yatılan katlarının her biri en az yarısı
  büyüklüğünde iki ya da daha çok yangın kompartımanına ayrılır ya da korunumlu yatay tahliye alanları kurulur (yatay
  tahliye alanında kullanıcı yükü 2,8 m²/kişi). Sağlık yapısı ``kullanim.siniflar``'ın okumasıyla; yatılan kat hasta
  odası, koğuş, süit ya da yatak odası olan kattır; katın alanı mahallerinin alanlarının toplamıdır (yönetmeliğin
  alanlarının ölçtüğü dış kontur, PAİY 4/1-ğ-1). Kompartımanlar ``kompartiman.kompartimanlar``'ın: aralarında en az
  24/7'nin süresi (60 dakika, 4/tt) beyan edilmiş, açık yeri olmayan ayrımlar; yalnız kaçış merdiveni mahallerinden
  oluşan kompartıman (merdivenin yuvası) sayılmaz. Korunumlu yatay tahliye alanı (35/7: en az 60 dakika dayanıklı, duman
  geçirmez) çizimde bu duvarlarla ayrılmış bir mahal olarak kompartıman sayılır. Tek kompartımanlı kat kaldı. İki ya da
  daha çok kompartımanda "en az yarısı büyüklüğünde" tek anlamla okunmuyor (her birinin katın yarısı olması ikiden çok
  kompartımanla olanaksız) ve yatay tahliye alanının 2,8 m²/kişi hesabı hangi kullanıcıların sığınacağına bağlı: bulgu
  kompartımanların alanlarıyla uzman incelemesi.
- **49/2 (19.12.2007):** hastanelerde koridor genişlikleri 2 m'den az olamaz. Hastane bina programının kullanımından
  (Hastane), o yoksa mahal adlarından (hasta odası, koğuş); huzurevi ve bakımevi hastane değildir. Koridor adı Koridor
  olan mahal; genişliği olguların en dar yeri (``least``): iki yanında kendi genişliğinde daire kalan en kısa kesit, net
  iç yüzlerden. Kapı girintisi gibi ağzından sığ cep sayılmaz; serbest duran kolonun yanındaki aralık ölçülmez.

**Okunan olgular** (``facts.building``): binaların kullanımı, katlar, mahaller (adı, katı, alanı, kaçış merdiveni olup
olmadığı, koridorun en dar yeri), her iki mahal arasındaki ayrımın duvarları, açıklıkları ve beyan edilen sınıfları,
katlar arası açıklıklar.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import yururluk as y
from .kompartiman import kompartimanlar
from .kullanim import _hasta_odasi, _otel_odasi, _sozcukler, _var, siniflar

MADDELER = [("BYKHY", "49/1-b"), ("BYKHY", "49/2"), ("BYKHY", "24/7")]

_HASTANE = ("HASTANE",)
_BAKIMEVI = ("HUZUREVI", "BAKIMEVI", "REHABILITASYON")


def _m(x: float) -> str:
    """Bir genişlik, santimetreye; milimetresi varsa milimetreye (1,999 m 2,00 diye yazılmaz)."""
    return (f"{x:.2f}" if abs(x * 100 - round(x * 100)) < 1e-6 else f"{x:.3f}").replace(".", ",")


def _m2(x: float) -> str:
    return f"{x:.0f}"


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _bulgu(code: str, ileti: str, onarim: str, hal: y.Hal, uids: list[str], iz: str,
           state: str | None = None) -> dict[str, Any]:
    out: dict[str, Any] = {"code": code, "message": f"{ileti} — {hal.kisa()}", "repair": onarim, "uids": uids,
                           "rule": hal.kural(), "_iz": iz}
    if state:
        out["state"] = state
    return out


def _bilinmiyor(madde: str, gun: date) -> dict[str, Any]:
    ilk = y.haller("BYKHY", madde)[0]
    return {"rule": f"BYKHY {madde}", "title": ilk.baslik, "version": None, "gazette": None, "state": y.UZMAN,
            "note": f"başvuru günü ({y.yaz(gun)}) kural setinin bildiği ilk hâlden ({ilk.etiket}) önce"}


def hastane(bina: dict[str, Any]) -> tuple[bool, str]:
    """Bina hastane mi (49/2) ve nereden okunduğu: bina programının kullanımında Hastane; kullanım yazılmamışsa hasta
    odası ya da koğuş adlı mahal. Huzurevi, bakımevi ve rehabilitasyon merkezi hastane değildir."""
    kullanim = list(bina.get("uses") or [])
    if kullanim:
        nereden = "bina programının kullanımı (“" + "”, “".join(kullanim) + "”)"
        return any(_var(_sozcukler(u), _HASTANE) for u in kullanim), nereden
    nereden = "mahallerin adları (bina programında kullanım yazılmamış)"
    return any(_var(_sozcukler(s.get("name")), ("HASTA", "KOGUS")) for s in bina.get("spaces") or []), nereden


#: Geçiş ve ıslak hacimler yatılan mahal değildir (Hasta koridoru, Hasta WC'si).
_YATILMAZ = ("KORIDOR", "HOL", "WC", "TUVALET", "BANYO", "DUS", "LAVABO", "BEKLEME")


def _yatilan(ad: str | None) -> bool:
    """Hastaların ya da bakılanların yattığı mahal: hasta odası, koğuş, süit, yatak odası; bakımevinin odası."""
    if _var(_sozcukler(ad), _YATILMAZ):
        return False
    return _hasta_odasi(ad) or _otel_odasi(ad)


def yatilan_katlar(bina: dict[str, Any], hal: y.Hal, dakika: int) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """49/1-b: 300 m²'den büyük yatılan katların kompartımanları."""
    sinir = float(hal.anahtar["area"])
    mahaller = bina.get("spaces") or []
    katlar = sorted(bina.get("storeys") or [], key=lambda s: float(s.get("elevation") or 0.0))
    gruplar = kompartimanlar(bina, dakika)
    bulgular: list[dict[str, Any]] = []
    olculen = []
    kalan = uzman = 0
    for k in katlar:
        seviye = k["level"]
        kattaki = [s for s in mahaller if s.get("level") == seviye]
        odalar = [s for s in kattaki if _yatilan(s.get("name"))]
        if not odalar:
            continue
        alan = sum(float(s.get("area") or 0.0) for s in kattaki)
        if alan <= sinir:
            continue
        olculen.append(k)
        uid_kat = {s["uid"] for s in kattaki}
        merdiven = {s["uid"] for s in kattaki if s.get("stair")}
        alanlar = {s["uid"]: float(s.get("area") or 0.0) for s in kattaki}
        bolumler = []
        for g in gruplar:
            burada = [u for u in g if u in uid_kat]
            if burada and any(u not in merdiven for u in burada):
                bolumler.append(sum(alanlar[u] for u in burada))
        ad = k.get("name") or seviye
        uids = sorted(s["uid"] for s in odalar)
        if len(bolumler) < 2:
            kalan += 1
            bulgular.append(_bulgu(
                "bykhy_patient_floor_compartments",
                f"{ad}: yatılan kat, alanı {_m2(alan)} m² > 300 m²; tek yangın kompartımanı. En az yarısı büyüklüğünde iki "
                "ya da daha çok kompartımana ayrılır ya da korunumlu yatay tahliye alanı kurulur",
                f"Katı en az EI {dakika} duvarlar ve kapılarla iki ya da daha çok kompartımana ayırın ya da korunumlu yatay "
                f"tahliye alanı (en az {dakika} dakika dayanıklı, duman geçirmez) kurun; duvarların sınıfını beyan edin.",
                hal, uids, f"{seviye}:kompartıman"))
        else:
            uzman += 1
            liste = ", ".join(f"{_m2(a)} m²" for a in sorted(bolumler, reverse=True))
            bulgular.append(_bulgu(
                "bykhy_patient_floor_compartments",
                f"{ad}: yatılan kat, alanı {_m2(alan)} m², {len(bolumler)} kompartıman ({liste}); “en az yarısı "
                "büyüklüğünde” şartı ve yatay tahliye alanının 2,8 m²/kişi hesabı çizimden okunmaz",
                "Kompartımanların büyüklüğünü ve sığınılacak kompartımanın ya da yatay tahliye alanının kapasitesini "
                "(2,8 m²/kişi) belgeleyin; idare karar verir.", hal, uids, f"{seviye}:büyüklük", y.UZMAN))
    if not olculen:
        return [], _satir(hal, y.KAPSAM_DISI, "300 m²'den büyük yatılan kat yok")
    if kalan:
        return bulgular, _satir(hal, y.KALDI, f"{kalan} yatılan kat tek kompartıman")
    return bulgular, _satir(hal, y.UZMAN, f"{uzman} yatılan kat iki ya da daha çok kompartıman; büyüklükleri idarenin")


def koridorlar(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """49/2: hastane koridorlarının genişliği."""
    sinir = float(hal.anahtar["width"])
    olanlar = [s for s in bina.get("spaces") or [] if any(w.startswith("KORIDOR") for w in _sozcukler(s.get("name")))]
    if not olanlar:
        return [], _satir(hal, y.KAPSAM_DISI, "adı Koridor olan mahal yok")
    bulgular: list[dict[str, Any]] = []
    kalan = eksik = 0
    for s in olanlar:
        ad = s.get("name") or "Koridor"
        en = s.get("least")
        if en is None:
            eksik += 1
            bulgular.append(_bulgu(
                "bykhy_hospital_corridor", f"“{ad}”: genişliği ölçülemedi, sınırı kaynaklarından bulunamıyor",
                "Mahalin sınırını duvarlarına göre yeniden bulun ya da sınırını çizin.", hal, [s["uid"]],
                f"{s['uid']}:ölçü", y.EKSIK_VERI))
        elif float(en) < sinir:
            kalan += 1
            bulgular.append(_bulgu(
                "bykhy_hospital_corridor", f"“{ad}”: en dar yeri {_m(float(en))} m < 2,00 m",
                "Koridoru en dar yerinde de en az 2,00 m olacak biçimde genişletin.", hal, [s["uid"]],
                f"{s['uid']}:en"))
    if kalan:
        return bulgular, _satir(hal, y.KALDI, f"{kalan} koridor 2,00 m'den dar")
    if eksik:
        return bulgular, _satir(hal, y.EKSIK_VERI, f"{eksik} koridorun genişliği ölçülemedi")
    en_dar = min(float(s["least"]) for s in olanlar)
    return [], _satir(hal, y.GECTI, f"{len(olanlar)} koridorun en dar yeri en az 2,00 m (en darı {_m(en_dar)} m)")


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin sağlık yapısı bulguları."""
    sinif_, nereden = siniflar(bina)
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    hal = y.hal("BYKHY", "49/1-b", gun)
    if hal is None:
        satirlar.append(_bilinmiyor("49/1-b", gun))
    elif "saglik" not in sinif_:
        satirlar.append(_satir(hal, y.KAPSAM_DISI, f"{nereden} hastane ya da bakımevi değil"))
    else:
        kompartiman = y.hal("BYKHY", "24/7", gun)
        dakika = int(kompartiman.anahtar.get("minutes", 60)) if kompartiman else 60
        b, s = yatilan_katlar(bina, hal, dakika)
        bulgular += b
        satirlar.append(s)
    hal = y.hal("BYKHY", "49/2", gun)
    if hal is None:
        satirlar.append(_bilinmiyor("49/2", gun))
    else:
        evet, nereden_h = hastane(bina)
        if not evet:
            satirlar.append(_satir(hal, y.KAPSAM_DISI, f"{nereden_h} hastane değil"))
        else:
            b, s = koridorlar(bina, hal)
            bulgular += b
            satirlar.append(s)
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin sağlık yapısı bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "sağlık yapısı kuralı")
