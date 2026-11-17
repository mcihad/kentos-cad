"""Yapı elemanlarının yangına dayanım süreleri: Binaların Yangından Korunması Hakkında Yönetmelik Ek-3/B ve Ek-3/C
(23, 24/1, 26/1, 29/7; docs/adr/8082, 8087).

Ek-3/C binanın kullanım sınıfına, yağmurlamasına ve elemanın yerine göre dakikayı verir: bodrum katlarda bodrumların
derinliğine (10 m'den fazla, az), giriş ve üst katlarda bina yüksekliğine (5, 21,50, 30,50 m'den az; 30,50'den fazla).
Ek-3/B elemanın neyi taşıması gerektiğini söyler. Bu kural müellifin beyan ettiği sınıfı istenenle karşılaştırır:

- **Taşıyıcı sistem** (kolon ve kiriş, Ek-3/B 1): R, Ek-3/C kadar.
- **Döşeme** (3): bir katın döşemesi (altındaki katın tavanı) REI, o katın Ek-3/C'si kadar; bodrumla zemin kat arası
  en az REI 90, bodrumların ve not (1) ile üst katların süresinin büyüğü; iki katlı bir ve iki ailelik evin ikinci katı
  REI 30. En alt katın döşemesi zemine oturur, okunmaz.
- **Dış duvar** (5): E ve I, Ek-3/C kadar. Not (3) taşıyıcı sistemin parçası olmayan elemanı 120'den 90'a indirir: duvarın
  taşıyıcı olup olmadığı çizimde yok, 90–120 arası beyan uzman incelemesi.
- **Kompartıman duvarı** (6, 7): E ve I, en az 60 ve Ek-3/C kadar.
- **Korunumlu yangın merdiveni yuvası** (9): binanın geri kalanından ayıran duvar REI 120, yangın güvenlik holüyle
  arasındaki REI 60.
- **Çatı** (4): döşeme görevi yapan çatı (teras döşemesi) REI, dıştan yangına maruz çatı EI, Ek-3/C kadar (katının
  sütunuyla). Kaçış yolu olan çatı bölümü (4-a) çizimde okunmaz. Asansör kat kapıları (12) şaft kuralındadır (``kuyular``,
  62/7).
- **İzin verilmez:** yağmurlamasız konut dışı bina 30,50 m'yi aşamaz.

Beyan yoksa eksik veri, okunmuyorsa eksik veri; yetmiyorsa kaldı. Bulgular kat, eleman türü ve sorun başına birdir.
Sınırda (bina yüksekliği tam 30,50 m, bodrum derinliği tam 10 m) tablonun iki sütunu da tutmaz: uzman incelemesi.

**Okunan olgular** (``facts.building``): elemanlar (``elements``: tür, kat, beyan, taşıyıcı sistemden mi, dış duvar mı),
katlar, ayrımlar ve mahaller (kompartıman ve yangın merdiveni duvarları), bina yüksekliği, bodrum derinliği,
kullanım, konut sayısı, yağmurlama.
"""

from __future__ import annotations

import re
from datetime import date
from typing import Any

from . import yururluk as y
from .kompartiman import ayiriyor
from .yangin_sinifi import okuma

MADDELER = [("BYKHY", "Ek-3")]

_TUR = {"column": ("kolon", "kolonun"), "beam": ("kiriş", "kirişin"), "wall": ("duvar", "duvarın"),
        "slab": ("döşeme", "döşemenin"), "roof": ("çatı", "çatının"), "terrace": ("teras döşemesi", "teras döşemesinin")}


def _katla(metin: str) -> str:
    tablo = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                           "ş": "S", "Ş": "S", "ü": "U", "Ü": "U", "â": "A", "Â": "A", "î": "I", "Î": "I"})
    return metin.translate(tablo).upper()


def _sozcukler(metin: str) -> list[str]:
    return [w for w in "".join(c if c.isalnum() else " " for c in _katla(metin or "")).split() if w]


def hucre(metin: str) -> dict[str, Any]:
    """Ek-3/C'nin bir hücresi: ``---`` yok, ``İzin verilmez``, ya da dakika ve notları (``120(3)``)."""
    t = metin.strip()
    if t == "---":
        return {"yok": True}
    if _katla(t).startswith("IZIN"):
        return {"izinsiz": True}
    m = re.match(r"^(\d+)((?:\(\d\))*)$", t)
    if not m:
        raise ValueError(f"Ek-3/C hücresi okunmadı: {metin}")
    return {"dakika": int(m.group(1)), "notlar": [int(n) for n in re.findall(r"\((\d)\)", m.group(2))]}


def _sinif_satiri(bina: dict[str, Any], hal: y.Hal) -> tuple[dict[str, Any] | None, str | None]:
    """Binanın Ek-3/C satırı; okunmuyorsa neden. Birden çok kullanımda dakikası en büyük olan sonra seçilir."""
    satirlar = hal.anahtar["rows"]
    bulunan = []
    for kullanim in bina.get("uses") or []:
        w = _sozcukler(kullanim)
        for s in satirlar:
            if any(x == k or (len(k) >= 4 and x.startswith(k)) for x in w for k in s.get("words", [])):
                if s not in bulunan:
                    bulunan.append(s)
                break
    konut, bolum = int(bina.get("dwellings") or 0), int(bina.get("units") or 0)
    # Bir ve iki ailelik ev: konuttan başka bölümü olmayan, en çok iki konutlu bina (Ek-3/C 1-a).
    ev = next(s for s in satirlar if s["id"] == "ev")
    if konut <= 2 and konut == bolum:
        bulunan = [ev if s["id"] == "apartman" else s for s in bulunan]
    if bulunan:
        return {"satirlar": bulunan}, None
    if konut > 0 and konut == bolum:
        satir = next(s for s in satirlar if s["id"] == ("ev" if konut <= 2 else "apartman"))
        return {"satirlar": [satir]}, None
    if bina.get("uses"):
        return None, "binanın kullanımı (“" + "”, “".join(bina["uses"]) + "”) Ek-3/C'nin sınıflarından birine okunmuyor"
    return None, "binanın kullanımı yazılmamış ve bölümleri yalnız konut değil"


def _sutun(yer: str, bina: dict[str, Any]) -> tuple[int | None, str]:
    """Ek-3/C'nin sütunu (0–5) ve sözü; tam sınırda yok (iki sütun da tutmaz)."""
    if yer == "bodrum":
        d = float(bina.get("basementDepth") or 0)
        if abs(d - 10.0) < 1e-9:
            return None, f"bodrum derinliği tam {d:.2f} m"
        return (0, f"bodrum derinliği {d:.2f} m (10 m'den fazla)") if d > 10.0 else (1, f"bodrum derinliği {d:.2f} m")
    h = float(bina.get("height") or 0)
    if abs(h - 30.5) < 1e-9:
        return None, f"bina yüksekliği tam {h:.2f} m"
    if h < 5.0:
        return 2, f"bina yüksekliği {h:.2f} m (5 m'den az)"
    if h < 21.5:
        return 3, f"bina yüksekliği {h:.2f} m (21,50 m'den az)"
    if h < 30.5:
        return 4, f"bina yüksekliği {h:.2f} m (30,50 m'den az)"
    return 5, f"bina yüksekliği {h:.2f} m (30,50 m'den fazla)"


def _istenen(sinif_: dict[str, Any], sutun: int, yagmurlama: bool) -> dict[str, Any]:
    """Binanın satırlarından o sütunun hücresi; birden çok satırda dakikası en büyük (izinsiz her şeyden önce)."""
    hucreler = []
    for s in sinif_["satirlar"]:
        dizi = s.get("all") or (s["wet"] if yagmurlama else s["dry"])
        hucreler.append(hucre(dizi[sutun]))
    if any(h.get("izinsiz") for h in hucreler):
        return {"izinsiz": True}
    sayili = [h for h in hucreler if "dakika" in h]
    if not sayili:
        return {"yok": True}
    return max(sayili, key=lambda h: h["dakika"])


def _kat_yeri(bina: dict[str, Any]) -> tuple[dict[str, str], dict[str, str], str | None, str | None]:
    """Her katın yeri (``bodrum`` ya da ``ust``) ve adı; zemin kat ve en alt kat."""
    katlar = list(bina.get("storeys") or [])
    if not katlar:
        return {}, {}, None, None
    zemin = next((k for k in katlar if "ZEMIN" in _katla(k.get("name") or "")), None) or min(
        katlar, key=lambda k: abs(float(k.get("elevation") or 0)))
    yer = {k["level"]: ("bodrum" if float(k["elevation"]) < float(zemin["elevation"]) - 1e-9 else "ust") for k in katlar}
    ad = {k["level"]: k.get("name") or k["level"] for k in katlar}
    en_alt = min(katlar, key=lambda k: float(k["elevation"]))["level"]
    return yer, ad, zemin["level"], en_alt


def _okur(beyan: str | None, harfler: str, dakika: int) -> bool | None:
    """Beyan istenen harflerle en az ``dakika`` mı; beyan yoksa ya da okunmuyorsa yok."""
    s = okuma(beyan)
    if s is None:
        return None
    return all(h in s["harf"] for h in harfler) and s["dakika"] >= dakika


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin yapı elemanı dayanım bulguları."""
    hal = y.hal("BYKHY", "Ek-3", gun)
    elemanlar = list(bina.get("elements") or [])
    if hal is None:
        if not elemanlar:
            return y.Sonuc([], [])
        ilk = y.haller("BYKHY", "Ek-3")[0]
        ileti = (f"Başvuru günü ({y.yaz(gun)}) BYKHY Ek-3'ün kural setindeki ilk hâlinden ({ilk.etiket}) önce: o günün "
                 f"metni KentOS'ta yok")
        return y.Sonuc([{"code": "regulation_text_unknown", "message": ileti, "repair": "Elemanların dayanımını "
                         "Ek-3'ün başvuru günündeki hâline göre uzman incelesin.",
                         "uids": sorted(e["uid"] for e in elemanlar)[:20], "rule": "BYKHY Ek-3", "state": y.UZMAN}],
                       [{"rule": "BYKHY Ek-3", "title": ilk.baslik, "version": None, "gazette": None,
                         "state": y.UZMAN, "note": ileti}])

    def satir(durum: str, not_: str) -> dict[str, Any]:
        return {"rule": "BYKHY Ek-3", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg, "state": durum,
                "note": not_}

    if not elemanlar and not bina.get("separations"):
        return y.Sonuc([], [satir(y.KAPSAM_DISI, "kolon, kiriş, duvar, döşeme ya da çatı yok")])
    sinif_, neden = _sinif_satiri(bina, hal)
    if sinif_ is None:
        return y.Sonuc([], [satir(y.EKSIK_VERI, neden or "")])
    yagmurlama = bool(bina.get("sprinklers"))
    adi = ", ".join(s["name"] for s in sinif_["satirlar"]) + (", yağmurlamalı" if yagmurlama else "")
    yer, kat_adi, zemin, en_alt = _kat_yeri(bina)
    bulgular: list[dict[str, Any]] = []
    # İzin verilmez: yağmurlamasız konut dışı bina 30,50 m'yi aşamaz.
    ust_sutun, ust_sozu = _sutun("ust", bina)
    if ust_sutun is not None and _istenen(sinif_, ust_sutun, yagmurlama).get("izinsiz"):
        bulgular.append({"code": "bykhy_height_not_permitted",
                         "message": f"Ek-3/C {adi} binaya {ust_sozu} izin vermez: yağmurlama sistemi gerekir — BYKHY Ek-3/C",
                         "repair": "Binaya yağmurlama sistemi kurun (Proje ayarları › Mimari › Yangın) ya da yüksekliği "
                                   "30,50 m'nin altına indirin.",
                         "uids": sorted(e["uid"] for e in elemanlar)[:20], "rule": hal.kural(), "_iz": "izinsiz"})
    # Her elemanın istediği: (harfler, dakika, gerekçe, uzman sınırı)
    istekler: dict[str, tuple[str, int, str, int | None]] = {}

    def ek3c(kat: str | None) -> tuple[dict[str, Any] | None, str]:
        sutun, soz = _sutun(yer.get(kat or "", "ust"), bina)
        if sutun is None:
            return None, soz
        return _istenen(sinif_, sutun, yagmurlama), soz

    belirsiz: list[str] = []
    for e in elemanlar:
        kat = e.get("level")
        h, soz = ek3c(kat)
        if h is None:
            belirsiz.append(e["uid"])
            continue
        if h.get("yok") or h.get("izinsiz"):
            continue
        dk, notlar = h["dakika"], h["notlar"]
        gerekce = f"Ek-3/C {adi}, {soz}"
        if e["kind"] in ("column", "beam") and e.get("frame"):
            istekler[e["uid"]] = ("R", dk, gerekce, None)
        elif e["kind"] == "slab":
            # The lowest storey's floor stands on the ground: no floor between two storeys.
            if kat == en_alt:
                continue
            bodrumlar = [k for k, v in yer.items() if v == "bodrum"]
            if kat == zemin and bodrumlar:
                alt, _ = ek3c(bodrumlar[0])
                dk = max(90, dk, (alt or {}).get("dakika", 0))
                gerekce = f"Ek-3/B 3-d bodrumla zemin kat arası en az REI 90, not (1) ile üst katlarınki; {gerekce}"
            elif (any(s["id"] == "ev" for s in sinif_["satirlar"])
                  and sum(1 for v in yer.values() if v == "ust") == 2 and yer.get(kat or "") == "ust" and kat != zemin):
                dk = 30
                gerekce = "Ek-3/B 3-a iki katlı konutun ikinci katı REI 30"
            istekler[e["uid"]] = ("REI", dk, gerekce, None)
        elif e["kind"] == "wall" and e.get("exterior"):
            istekler[e["uid"]] = ("EI", dk, f"Ek-3/B 5 dış duvar; {gerekce}", 90 if 3 in notlar and dk == 120 else None)
        elif e["kind"] == "terrace":
            istekler[e["uid"]] = ("REI", dk, f"Ek-3/B 4-b döşeme görevi yapan çatı; {gerekce}", None)
        elif e["kind"] == "roof":
            istekler[e["uid"]] = ("EI", dk, f"Ek-3/B 4-c dıştan yangına maruz çatı; {gerekce}", None)
    # Kompartıman duvarları (6, 7) ve korunumlu yangın merdiveni yuvası (9): ayrımlardan.
    adlar = {m["uid"]: _katla(m.get("name") or "") for m in bina.get("spaces") or []}

    def merdiven(u: str) -> bool:
        a = adlar.get(u, "")
        return ("YANGIN" in a and "MERDIVEN" in a) or "KORUNUMLU" in a

    def hol(u: str) -> bool:
        a = adlar.get(u, "")
        return "GUVENLIK" in a and "HOL" in a

    for a in bina.get("separations") or []:
        x, z = a["rooms"]
        duvarlar = list(a.get("walls") or [])
        if merdiven(x) != merdiven(z):
            diger = z if merdiven(x) else x
            dk, ne = (60, "Ek-3/B 9-b yangın merdiveni ile yangın güvenlik holü arası") if hol(diger) else (
                120, "Ek-3/B 9-a korunumlu yangın merdiveni yuvasını binanın geri kalanından ayıran duvar")
            for w in duvarlar:
                istekler[w["uid"]] = ("REI", max(dk, istekler.get(w["uid"], ("", 0, "", None))[1]), ne, None)
        elif ayiriyor(a, 60):
            h, soz = ek3c(a.get("level"))
            dk = max(60, (h or {}).get("dakika", 0))
            for w in duvarlar:
                if w["uid"] not in istekler:
                    istekler[w["uid"]] = ("EI", dk, f"Ek-3/B 6–7 kompartıman duvarı en az EI 60; Ek-3/C {adi}, {soz}",
                                          None)
    # Karşılaştırma: kat, tür ve sorun başına bir bulgu.
    turu = {e["uid"]: e for e in elemanlar}
    for a in bina.get("separations") or []:
        for w in a.get("walls") or []:
            turu.setdefault(w["uid"], {"uid": w["uid"], "kind": "wall", "level": a.get("level"),
                                       "fireRating": w.get("fireRating")})
    gruplar: dict[tuple, list[str]] = {}
    for uid, (harf, dk, gerekce, uzman_siniri) in istekler.items():
        e = turu.get(uid)
        if e is None:
            continue
        okunan = _okur(e.get("fireRating"), harf, dk)
        if okunan:
            continue
        if okunan is None:
            anahtar = (e.get("level"), e["kind"], "eksik", harf, dk, gerekce, None)
        elif uzman_siniri is not None and _okur(e.get("fireRating"), harf, uzman_siniri):
            anahtar = (e.get("level"), e["kind"], "uzman", harf, dk, gerekce, e.get("fireRating"))
        else:
            anahtar = (e.get("level"), e["kind"], "yetmiyor", harf, dk, gerekce, e.get("fireRating"))
        gruplar.setdefault(anahtar, []).append(uid)
    for (kat, tur, sorun, harf, dk, gerekce, beyan), uids in sorted(gruplar.items(), key=lambda t: str(t[0])):
        ad, adin = _TUR.get(tur, ("eleman", "elemanın"))
        yerde = kat_adi.get(kat or "", "Katsız")
        sayi = len(uids)
        if sorun == "eksik":
            ileti = f"{yerde}: {sayi} {adin} yangın sınıfı beyan edilmemiş; {gerekce} en az {harf} {dk} ister"
            durum = y.EKSIK_VERI
        elif sorun == "uzman":
            ileti = (f"{yerde}: {sayi} {ad} {beyan}; {gerekce} {harf} {dk} ister, taşıyıcı sistemin parçası değilse not (3) "
                     f"90'a indirir: duvarın taşıyıcı olup olmadığı çizimde yok")
            durum = y.UZMAN
        else:
            ileti = f"{yerde}: {sayi} {ad} {beyan}; {gerekce} en az {harf} {dk} ister"
            durum = None
        b = {"code": "bykhy_element_resistance", "message": f"{ileti} — BYKHY Ek-3", "repair":
             "Elemanların (ya da stillerinin, tiplerinin) yangın sınıfını istenen dakikayla beyan edin (Öznitelikler › "
             "Yangın dayanımı) ya da elemanı güçlendirin.", "uids": sorted(uids), "rule": hal.kural(),
             "_iz": f"{kat}:{tur}:{sorun}:{harf}{dk}:{beyan}"}
        if durum:
            b["state"] = durum
        bulgular.append(b)
    if belirsiz:
        bulgular.append({"code": "bykhy_element_resistance", "state": y.UZMAN,
                         "message": "Ek-3/C'nin iki sütununun sınırında (bina yüksekliği tam 30,50 m ya da bodrum "
                                    "derinliği tam 10 m): tablo hangisini istediğini söylemiyor — BYKHY Ek-3",
                         "repair": "Elemanların süresine uzman karar versin.", "uids": sorted(belirsiz)[:20],
                         "rule": hal.kural(), "_iz": "sınır"})
    kaldi = [b for b in bulgular if "state" not in b]
    if any(b["code"] == "bykhy_height_not_permitted" for b in bulgular):
        durum = satir(y.KALDI, f"yağmurlamasız {adi} binaya {ust_sozu} izin verilmez")
    elif kaldi:
        durum = satir(y.KALDI, f"{len(kaldi)} eleman grubu Ek-3'ün süresini tutmuyor ({adi})")
    elif any(b.get("state") == y.EKSIK_VERI for b in bulgular):
        durum = satir(y.EKSIK_VERI, f"elemanların yangın sınıfı beyan edilmemiş ({adi})")
    elif bulgular:
        durum = satir(y.UZMAN, "taşıyıcı olup olmadığı ya da tablonun sınırı uzman incelemesi")
    elif not istekler:
        durum = satir(y.KAPSAM_DISI, "Ek-3'ün süre istediği eleman yok (taşıyıcı sistem, döşeme, çatı, dış ya da "
                                     "kompartıman duvarı)")
    else:
        durum = satir(y.GECTI, f"{len(istekler)} eleman Ek-3'ün süresini tutuyor ({adi})")
    return y.Sonuc(bulgular, [durum])


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin yapı elemanı dayanım bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "yangın dayanımı kuralı")
