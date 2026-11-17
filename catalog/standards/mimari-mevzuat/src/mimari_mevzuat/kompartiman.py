"""Yangın kompartımanları: Binaların Yangından Korunması Hakkında Yönetmelik 4/tt, 24/2, 24/6 ve Ek-4, 24/7
(docs/adr/8081).

- **Kompartıman** (4/tt): her yanı en az 60 dakika yangına dayanıklı elemanlarla duman ve ısı geçirmez ayrılmış bölge.
  Bütün döşemeler yangın duvarı niteliğindedir (26/1): bir katın kompartımanları o katın içindedir. İki mahal, aralarında
  açık bir yer (mahal ayırıcı, boşluk) ya da en az EI 60 beyan edilmemiş bir duvar ya da açıklık varsa aynı
  kompartımandadır. Katlar arası açıklıklar (galeri, atrium, açık merdiven) okunmaz.
- **24/2:** kazan dairesi, otopark, ana elektrik dağıtım odası, trafo, orta gerilim, jeneratör odasının duvarları
  kompartıman duvarı niteliğindedir: başka bir mahalle arasındaki her şey en az EI 60.
- **24/6, Ek-4:** kompartımanın alanı binanın kullanım sınıfının en çoğunu aşmaz; konut sınırsız. Yağmurlama varsa
  (uygun yangın kontrol sistemi) not (1) iki katına çıkarır, not (2) ve (3) sınırsız yapar. Sanayi ve depoda sınır
  tehlike sınıfına (Ek-1) bağlı: çizim söylemez, uzman incelemesi.
- **24/7:** kompartımanlar arasında yangına dayanıksız açıklık bulunamaz: duvarları en az EI 60 beyan edilmiş bir
  ayrımda beyansız ya da yetmeyen bir kapı, pencere, boşluk ya da açık yer bulgu.

**Okunan olgular** (``facts.building``): mahaller ve alanları (``spaces``), her iki mahal arasındaki ayrım
(``separations``: duvarlar, açıklıklar, beyan edilen sınıfları ve boyları, açık boy), katlar, bağımsız bölüm ve konut
sayısı, binaların kullanımı, yağmurlama.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import yururluk as y
from .yangin_sinifi import ayirir

MADDELER = [("BYKHY", "24/2"), ("BYKHY", "24/6"), ("BYKHY", "24/7")]

#: 24/2'nin yangın tehlikesi olan kapalı alanlarının adlarındaki sözcükler.
_TEHLIKELI = ("KAZAN", "OTOPARK", "GARAJ", "ELEKTRIK", "TRAFO", "JENERATOR")

#: Bir açık yer bundan kısaysa sayılmaz (çizimin köşe gürültüsü), m.
_EN_AZ = 0.05


def _katla(metin: str) -> str:
    tablo = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                           "ş": "S", "Ş": "S", "ü": "U", "Ü": "U", "â": "A", "Â": "A", "î": "I", "Î": "I"})
    return metin.translate(tablo).upper()


def _sozcukler(metin: str) -> list[str]:
    return [w for w in "".join(c if c.isalnum() else " " for c in _katla(metin or "")).split() if w]


def tehlikeli(ad: str) -> bool:
    """Bir mahal adı 24/2'nin yangın tehlikesi olan kapalı alanı mı (kazan dairesi, otopark, elektrik, trafo, orta gerilim,
    jeneratör)."""
    w = _sozcukler(ad)
    if any(x.startswith(k) for x in w for k in _TEHLIKELI):
        return True
    return any(a.startswith("ORTA") and b.startswith("GERILIM") for a, b in zip(w, w[1:]))


def _m2(x: float) -> str:
    return f"{x:.2f}"


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


def _ogeler(ayrim: dict[str, Any]) -> list[dict[str, Any]]:
    return list(ayrim.get("walls") or []) + list(ayrim.get("openings") or [])


def ayiriyor(ayrim: dict[str, Any], dakika: int) -> bool:
    """Bir ayrım bir kompartıman sınırı mı: açık yeri yok, her duvarı ve açıklığı en az ``dakika`` EI."""
    if float(ayrim.get("open") or 0) > _EN_AZ:
        return False
    ogeler = _ogeler(ayrim)
    return bool(ogeler) and all(ayirir(o.get("fireRating"), dakika) for o in ogeler)


def kompartimanlar(bina: dict[str, Any], dakika: int) -> list[list[str]]:
    """Her katın kompartımanları: mahallerin kimlikleri, sınır olmayan ayrımlarla birleşmiş."""
    ata: dict[str, str] = {}

    def kok(x: str) -> str:
        while ata.setdefault(x, x) != x:
            ata[x] = ata[ata[x]]
            x = ata[x]
        return x

    mahaller = [m["uid"] for m in bina.get("spaces") or []]
    for m in mahaller:
        kok(m)
    for a in bina.get("separations") or []:
        if not ayiriyor(a, dakika):
            x, z = kok(a["rooms"][0]), kok(a["rooms"][1])
            if x != z:
                ata[max(x, z)] = min(x, z)
    gruplar: dict[str, list[str]] = {}
    for m in mahaller:
        gruplar.setdefault(kok(m), []).append(m)
    return sorted(gruplar.values())


def _sinifi(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], str | None]:
    """Binanın Ek-4 kullanım sınıfları (adlarıyla); bilinmiyorsa neden."""
    siniflar = hal.anahtar["classes"]
    bulunan = []
    for kullanim in bina.get("uses") or []:
        w = _sozcukler(kullanim)
        for s in siniflar:
            if any(x == k or (len(k) >= 4 and x.startswith(k)) for x in w for k in s["words"]):
                if s not in bulunan:
                    bulunan.append(s)
                break
    if bulunan:
        return bulunan, None
    konut, bolum = int(bina.get("dwellings") or 0), int(bina.get("units") or 0)
    if konut > 0 and konut == bolum:
        return [next(s for s in siniflar if s["id"] == "konut")], None
    if bina.get("uses"):
        return [], "binanın kullanımı (“" + "”, “".join(bina["uses"]) + "”) Ek-4'ün sınıflarından birine okunmuyor"
    return [], "binanın kullanımı yazılmamış ve bölümleri yalnız konut değil"


def alan(bina: dict[str, Any], hal: y.Hal, dakika: int) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """24/6 ve Ek-4: kompartımanın en çok alanı."""
    siniflar, neden = _sinifi(bina, hal)
    if neden:
        return [], _satir(hal, y.EKSIK_VERI, neden)
    uzman = [s for s in siniflar if s.get("limit") is None and not s.get("unlimited")]
    if uzman:
        return [], _satir(hal, y.UZMAN, "sınırı tehlike sınıfına bağlı (Ek-1): " + ", ".join(s["name"] for s in uzman))
    yagmurlama = bool(bina.get("sprinklers"))

    def sinir(s: dict[str, Any]) -> float | None:
        if s.get("unlimited"):
            return None
        if yagmurlama and s.get("note") in (2, 3):
            return None
        return float(s["limit"]) * (2 if yagmurlama and s.get("note") == 1 else 1)

    sinirlar = [(sinir(s), s) for s in siniflar]
    sayili = [(x, s) for x, s in sinirlar if x is not None]
    if not sayili:
        adlar = ", ".join(s["name"] for s in siniflar)
        return [], _satir(hal, y.GECTI, f"{adlar}: kompartıman alanı sınırsız" + (" (yağmurlama)" if yagmurlama else ""))
    en_az, s = min(sayili, key=lambda t: t[0])
    alanlar = {m["uid"]: float(m.get("area") or 0) for m in bina.get("spaces") or []}
    adlar = {m["uid"]: m.get("name") or "Mahal" for m in bina.get("spaces") or []}
    bulgular = []
    gruplar = kompartimanlar(bina, dakika)
    for g in gruplar:
        toplam = sum(alanlar.get(u, 0.0) for u in g)
        if toplam > en_az + 1e-6:
            bulgular.append(_bulgu(
                "bykhy_compartment_area_over",
                f"Kompartıman {_m2(toplam)} m² ({', '.join(adlar[u] for u in g[:4])}{' …' if len(g) > 4 else ''}): "
                f"Ek-4'ün {s['name']} sınırı {en_az:.0f} m²",
                "Kompartımanı en az EI 60 duvar ve kapılarla bölün ya da yağmurlama gibi bir yangın kontrol sistemi "
                "kurun (Ek-4'ün notları).", hal, sorted(g), "alan:" + ",".join(sorted(g))))
    en_buyuk = max((sum(alanlar.get(u, 0.0) for u in g) for g in gruplar), default=0.0)
    if bulgular:
        return bulgular, _satir(hal, y.KALDI, f"{len(bulgular)} kompartıman {s['name']} sınırını ({en_az:.0f} m²) aşıyor")
    return [], _satir(hal, y.GECTI, f"{len(gruplar)} kompartıman, en büyüğü {_m2(en_buyuk)} m² (sınır {en_az:.0f} m², "
                                    f"{s['name']})")


def tehlike(bina: dict[str, Any], hal: y.Hal, dakika: int) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """24/2: yangın tehlikesi olan kapalı alanların duvarları kompartıman duvarı niteliğinde."""
    adlar = {m["uid"]: m.get("name") or "Mahal" for m in bina.get("spaces") or []}
    riskli = {u for u, a in adlar.items() if tehlikeli(a)}
    if not riskli:
        return [], _satir(hal, y.KAPSAM_DISI, "kazan dairesi, otopark, elektrik, trafo ya da jeneratör odası yok")
    bulgular: list[dict[str, Any]] = []
    for a in bina.get("separations") or []:
        x, z = a["rooms"]
        for oda, komsu in ((x, z), (z, x)):
            if oda not in riskli:
                continue
            eksik = [o for o in _ogeler(a) if ayirir(o.get("fireRating"), dakika) is None]
            yetmeyen = [o for o in _ogeler(a) if ayirir(o.get("fireRating"), dakika) is False]
            acik = float(a.get("open") or 0) > _EN_AZ
            if not (eksik or yetmeyen or acik):
                continue
            neler = []
            if acik:
                neler.append(f"{float(a['open']):.2f} m açık (duvar yok)")
            neler += [f"{_tur(o)} {o.get('fireRating')} (en az EI {dakika})" for o in yetmeyen]
            neler += [f"{_tur(o)} sınıfı beyan edilmemiş" for o in eksik]
            durum = y.EKSIK_VERI if eksik and not (yetmeyen or acik) else None
            bulgular.append(_bulgu(
                "bykhy_hazard_room_separation",
                f"“{adlar.get(oda, 'Mahal')}” ile “{adlar.get(komsu, 'Mahal')}” arasında: " + "; ".join(neler)
                + f"; 24/2 kompartıman duvarı niteliğinde ister",
                "Ayrımın duvarlarını ve kapılarını en az EI 60 yapın ve sınıfını beyan edin (Öznitelikler › Yangın "
                "dayanımı).", hal, [oda, komsu] + [o["uid"] for o in eksik + yetmeyen], f"{oda}:{komsu}", durum))
            break
    if any("state" not in b for b in bulgular):
        satir = _satir(hal, y.KALDI, f"{len(riskli)} tehlikeli alanın duvarı kompartıman duvarı değil")
    elif bulgular:
        satir = _satir(hal, y.EKSIK_VERI, "tehlikeli alanların duvar ya da kapı sınıfı beyan edilmemiş")
    else:
        satir = _satir(hal, y.GECTI, f"{len(riskli)} tehlikeli alan en az EI {dakika} ile ayrılmış")
    return bulgular, satir


def _tur(o: dict[str, Any]) -> str:
    return {"wall": "duvar", "door": "kapı", "window": "pencere", "gap": "boşluk"}.get(o.get("kind"), "eleman")


def aciklik(bina: dict[str, Any], hal: y.Hal, dakika: int) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """24/7: kompartıman duvarında yangına dayanıksız açıklık."""
    adlar = {m["uid"]: m.get("name") or "Mahal" for m in bina.get("spaces") or []}
    duvarli = [a for a in bina.get("separations") or []
               if a.get("walls") and all(ayirir(w.get("fireRating"), dakika) for w in a["walls"])]
    if not duvarli:
        return [], _satir(hal, y.KAPSAM_DISI, f"duvarları en az EI {dakika} beyan edilmiş ayrım yok")
    bulgular: list[dict[str, Any]] = []
    for a in duvarli:
        x, z = a["rooms"]
        arada = f"“{adlar.get(x, 'Mahal')}” ile “{adlar.get(z, 'Mahal')}” arasındaki kompartıman duvarında"
        for o in a.get("openings") or []:
            okunan = ayirir(o.get("fireRating"), dakika)
            if okunan:
                continue
            neden = (f"{o.get('fireRating')} (en az EI {dakika})" if okunan is False
                     else "sınıfı beyan edilmemiş")
            bulgular.append(_bulgu(
                "bykhy_compartment_opening", f"{arada} {_tur(o)}: {neden}; 24/7 yangına dayanıksız açıklığa izin vermez",
                "Açıklığı en az EI 60 bir kapı ya da pencereyle kapatın ve sınıfını beyan edin.", hal,
                [o["uid"], x, z], f"{o['uid']}", y.EKSIK_VERI if okunan is None else None))
        if float(a.get("open") or 0) > _EN_AZ:
            bulgular.append(_bulgu(
                "bykhy_compartment_opening", f"{arada} {float(a['open']):.2f} m açık yer (duvar yok)",
                "Duvarı sürdürün ya da açık yeri en az EI 60 bir elemanla kapatın.", hal, [x, z], f"{x}:{z}:açık"))
    if any("state" not in b for b in bulgular):
        satir = _satir(hal, y.KALDI, f"{len(bulgular)} açıklık kompartıman duvarını bozuyor")
    elif bulgular:
        satir = _satir(hal, y.EKSIK_VERI, "kompartıman duvarındaki açıklıkların sınıfı beyan edilmemiş")
    else:
        satir = _satir(hal, y.GECTI, f"{len(duvarli)} kompartıman duvarındaki açıklıklar en az EI {dakika}")
    return bulgular, satir


def _bilinmiyor(madde: str, gun: date, uids: list[str]) -> tuple[dict[str, Any], dict[str, Any]]:
    """Başvuru günü maddenin kural setindeki ilk hâlinden önce: uzman incelemesi."""
    ilk = y.haller("BYKHY", madde)[0]
    ileti = (f"Başvuru günü ({y.yaz(gun)}) BYKHY {madde} maddesinin kural setindeki ilk hâlinden ({ilk.etiket}) önce: o "
             f"günün metni KentOS'ta yok")
    bulgu = {"code": "regulation_text_unknown", "message": ileti, "repair": "Kompartımanları maddenin başvuru günündeki "
             "metnine göre uzman incelesin.", "uids": uids, "rule": f"BYKHY {madde}", "state": y.UZMAN}
    return bulgu, {"rule": f"BYKHY {madde}", "title": ilk.baslik, "version": None, "gazette": None, "state": y.UZMAN,
                   "note": ileti}


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin kompartıman bulguları."""
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    for madde, denetim in (("24/2", tehlike), ("24/6", alan), ("24/7", aciklik)):
        hal = y.hal("BYKHY", madde, gun)
        if hal is None:
            b, s = _bilinmiyor(madde, gun, sorted(m["uid"] for m in bina.get("spaces") or [])[:20])
            bulgular.append(b)
            satirlar.append(s)
            continue
        dakika = int(hal.anahtar.get("minutes", 60))
        b, s = denetim(bina, hal, dakika)
        bulgular += b
        satirlar.append(s)
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin kompartıman bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys") or not bina.get("spaces"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "kompartıman kuralı")
