"""Komşu binaya bitişik duvar (docs/adr/8142): Planlı Alanlar İmar Yönetmeliği 39/4, Binaların Yangından Korunması
Hakkında Yönetmelik 25/1 ve 25/2.

Vaziyet planının kenarı "Bitişik (komşu bina)" ise bina orada komşu parseldeki binaya bitişiktir (PAİY 4/1-pppp-2, -3);
KentOS o kenar boyunca, her katta, parselin 5 cm içinde duran duvarları ve açıklıkları beyan edilen sınıflarıyla ve
uzunluklarıyla verir (``building.party``). Ayrık nizamda kenar yan bahçedir: okunmaz.

- **PAİY 39/4 (1.10.2017):** bitişik ve blok nizama tabi binalarda komşu parsel sınırı üzerindeki bitişik duvarlarda
  pencere ve kapı açılamaz. Bitişik kenardaki pencere ve kapı kaldı. Bitişik kenar binanın bitişik ya da blok (ikiz)
  nizamda olduğunu söyler: yapı nizamı yazılmamış olsa da madde uygulanır.
- **BYKHY 25/1 (19.12.2007):** bitişik nizam yapıları birbirinden ayıran yangın duvarları yangına en az 90 dakika
  dayanıklı: bitişik kenardaki duvarın beyan edilen sınıfı bütünlük (E) ve yalıtımla (I), taşıyıcı beyan edilmişse
  taşıyıcılıkla (R) da en az 90. Sınıf beyan edilmemişse eksik veri, azsa kaldı.
- **BYKHY 25/2:** yangın duvarlarında delik ve boşluk bulunamaz; kapı ve sabit ışık penceresi gibi boşluklardan
  kaçınılamıyorsa en az yangın duvarının süresinin yarısı dayanıklı, kapı kendiliğinden kapanan ve duman sızdırmaz.
  Bitişik kenardaki boşluk (kapısız, penceresiz açıklık) kaldı; sınıfı yarı süreye yetmeyen kapı ve pencere kaldı,
  sınıfı beyan edilmemişse eksik veri; yeten kapının sınıfında kendiliğinden kapanma (C) ya da duman sızdırmazlık (S)
  yoksa eksik veri; ikisi de olan kapının ve yeten pencerenin kaçınılmazlığı (pencerenin sabit olduğu da) uzman
  incelemesi.

**Okunan olgular** (``facts.building``): ``party`` (kenar, kat, duvarlar ve açıklıklar: türü, beyan edilen sınıfı,
uzunluğu), ``elements`` (duvarın taşıyıcılık beyanı), katların adları.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import yururluk as y
from .yangin_sinifi import okuma

MADDELER = [("PAİY", "39/4"), ("BYKHY", "25/1"), ("BYKHY", "25/2")]

_TUR = {"door": "kapı", "window": "pencere", "gap": "boşluk"}


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


def _sonuc(hal: y.Hal, bulgular: list[dict[str, Any]], kaldi: str, eksik: str, gecti: str
           ) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """Maddenin satırı bulgularından: biri kaldıysa kaldı, eksik veri varsa o, yalnız uzman incelemesiyse o, yoksa
    geçti."""
    if any("state" not in b for b in bulgular):
        return bulgular, _satir(hal, y.KALDI, kaldi)
    if any(b.get("state") == y.EKSIK_VERI for b in bulgular):
        return bulgular, _satir(hal, y.EKSIK_VERI, eksik)
    if bulgular:
        return bulgular, _satir(hal, y.UZMAN, eksik)
    return bulgular, _satir(hal, y.GECTI, gecti)


def _yer(bina: dict[str, Any], p: dict[str, Any]) -> str:
    """“2. kenar (bitişik), 1. kat”."""
    adlar = {s.get("level"): s.get("name") for s in bina.get("storeys") or []}
    kat = adlar.get(p.get("level")) or p.get("level") or "katsız"
    return f"{p['edge']}. kenar (bitişik), {kat}"


def _bitisikler(bina: dict[str, Any]) -> list[dict[str, Any]]:
    return list(bina.get("party") or [])


def pencereler(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """39/4: bitişik kenardaki duvarda pencere ve kapı yok."""
    hepsi = _bitisikler(bina)
    if not hepsi:
        return [], _satir(hal, y.KAPSAM_DISI, "komşu binaya bitişik duvar yok")
    bulgular = []
    for p in hepsi:
        acik = [o for o in p.get("openings") or [] if o.get("kind") in ("door", "window")]
        if not acik:
            continue
        sayi = {}
        for o in acik:
            sayi[o["kind"]] = sayi.get(o["kind"], 0) + 1
        ne = " ve ".join(f"{sayi[k]} {_TUR[k]}" for k in ("window", "door") if k in sayi)
        bulgular.append(_bulgu(
            "paiy_party_wall_opening",
            f"{_yer(bina, p)}: komşu parsel sınırındaki bitişik duvarda {ne}; 39/4 bitişik ve blok nizamda bu duvarda "
            f"pencere ve kapıya izin vermez",
            "Pencereyi ve kapıyı bitişik duvardan kaldırın; bina orada komşuya bitişik değilse kenarın türünü Parsel "
            "aracında değiştirin.",
            hal, sorted(o["uid"] for o in acik), f"{p['edge']}:{p.get('level')}:39/4"))
    duvar = sum(len(p.get("walls") or []) for p in hepsi)
    return _sonuc(hal, bulgular, "bitişik duvarda pencere ya da kapı var", "",
                  f"{duvar} bitişik duvarda pencere ve kapı yok")


def yangin_duvari(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """25/1: bitişik kenardaki duvar en az 90 dakika (E ve I; taşıyıcı beyan edilmişse R de)."""
    dk = int(hal.anahtar["minutes"])
    tasiyici = {e["uid"]: e.get("loadBearing") is True for e in bina.get("elements") or []}
    gorulen: set[str] = set()
    bulgular = []
    duvarlar = 0
    for p in _bitisikler(bina):
        for w in p.get("walls") or []:
            if w["uid"] in gorulen:
                continue
            gorulen.add(w["uid"])
            duvarlar += 1
            istenen = ("R", "E", "I") if tasiyici.get(w["uid"]) else ("E", "I")
            ad = "".join(istenen)
            sinif = w.get("fireRating")
            o = okuma(sinif)
            if o is None:
                neden, durum = "yangın dayanım sınıfı beyan edilmemiş", y.EKSIK_VERI
            elif all(h in o["harf"] for h in istenen) and o["dakika"] >= dk:
                continue
            else:
                neden, durum = f"{sinif}, en az {ad} {dk} değil", None
            bulgular.append(_bulgu(
                "bykhy_party_fire_wall",
                f"{_yer(bina, p)}: komşu binayla arasındaki duvarın {neden}; 25/1 bitişik nizam yapılarını ayıran "
                f"yangın duvarını en az {dk} dakika ister",
                f"Duvarın yangın dayanım sınıfını en az {ad} {dk} beyan edin (Öznitelikler › Yangın dayanımı).",
                hal, [w["uid"]], f"{w['uid']}:25/1", durum))
    if not duvarlar:
        return [], _satir(hal, y.KAPSAM_DISI, "komşu binaya bitişik duvar yok")
    return _sonuc(hal, bulgular, "bitişik duvar yangın duvarı değil", "bitişik duvarın sınıfı okunmuyor",
                  f"{duvarlar} bitişik duvar en az {dk} dakika")


def bosluklar(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """25/2: yangın duvarında delik ve boşluk yok; kaçınılamayan kapı ve sabit ışık penceresi en az yarı süre, kapı
    kendiliğinden kapanan ve duman sızdırmaz."""
    yari = int(round(float(hal.anahtar["minutes"]) * float(hal.anahtar["fraction"])))
    hepsi = _bitisikler(bina)
    if not any(p.get("walls") or p.get("openings") for p in hepsi):
        return [], _satir(hal, y.KAPSAM_DISI, "komşu binaya bitişik duvar yok")
    bulgular = []
    for p in hepsi:
        for o in p.get("openings") or []:
            tur = _TUR.get(o.get("kind"), "boşluk")
            sinif = o.get("fireRating")
            s = okuma(sinif)
            if o.get("kind") not in ("door", "window"):
                neden, durum = "kapısız, penceresiz bir boşluk", None
            elif s is None:
                neden, durum = "yangın dayanım sınıfı beyan edilmemiş", y.EKSIK_VERI
            elif not ("E" in s["harf"] and s["dakika"] >= yari):
                neden, durum = f"{sinif}, en az E {yari} değil", None
            elif o.get("kind") == "door" and not (s["kapanma"] and s["duman"]):
                eksik = " ve ".join(n for n, var in (("kendiliğinden kapanma (C)", s["kapanma"]),
                                                     ("duman sızdırmazlık (S)", s["duman"])) if not var)
                neden, durum = f"{sinif}: sınıfında {eksik} yazılı değil", y.EKSIK_VERI
            else:
                neden = (f"{sinif}: süre yeter; boşluktan kaçınılamadığı"
                         + (", pencerenin sabit olduğu" if o.get("kind") == "window" else "") + " uzman incelemesi")
                durum = y.UZMAN
            bulgular.append(_bulgu(
                "bykhy_party_wall_opening",
                f"{_yer(bina, p)}: komşu binayla arasındaki yangın duvarında {tur}, {neden}; 25/2 yangın duvarında "
                f"delik ve boşluk istemez, kaçınılamayan kapı ve sabit ışık penceresine en az yarı süre (E {yari}) "
                f"ister",
                "Açıklığı yangın duvarından kaldırın; kaçınılamıyorsa en az E "
                f"{yari}, kendiliğinden kapanan ve duman sızdırmaz (C, S) bir kapı ya da sabit ışık penceresi beyan edin.",
                hal, [o["uid"]], f"{o['uid']}:25/2", durum))
    return _sonuc(hal, bulgular, "yangın duvarında açıklık var", "yangın duvarındaki açıklığın sınıfı ya da uzman "
                  "incelemesi", "bitişik duvarlarda açıklık yok")


#: Her maddenin denetimi.
_DENETIMLER = (("PAİY", "39/4", pencereler), ("BYKHY", "25/1", yangin_duvari), ("BYKHY", "25/2", bosluklar))


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin bitişik duvar bulguları."""
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    for yonetmelik, madde, denetim in _DENETIMLER:
        hal = y.hal(yonetmelik, madde, gun)
        if hal is None:
            uids = sorted({e["uid"] for p in _bitisikler(bina) for e in (p.get("walls") or []) + (p.get("openings")
                                                                                                  or [])})
            if not uids:
                continue
            ilk = y.haller(yonetmelik, madde)[0]
            ileti = (f"Başvuru günü ({y.yaz(gun)}) {yonetmelik} {madde} maddesinin kural setindeki ilk hâlinden "
                     f"({ilk.etiket}) önce: o günün metni KentOS'ta yok")
            bulgular.append({"code": "regulation_text_unknown", "message": ileti,
                             "repair": "Bitişik duvarları maddenin başvuru günündeki metnine göre uzman incelesin.",
                             "uids": uids[:20], "rule": f"{yonetmelik} {madde}", "_iz": f"{madde}:bilinmiyor",
                             "state": y.UZMAN})
            satirlar.append({"rule": f"{yonetmelik} {madde}", "title": ilk.baslik, "version": None, "gazette": None,
                             "state": y.UZMAN, "note": ileti})
            continue
        b, s = denetim(bina, hal)
        bulgular += b
        satirlar.append(s)
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin bitişik duvar bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "bitişik duvar kuralı")
