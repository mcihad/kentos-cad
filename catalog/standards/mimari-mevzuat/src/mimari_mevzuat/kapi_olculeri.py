"""Kapı ölçüleri: Planlı Alanlar İmar Yönetmeliği 39/1 ve Sivas İmar Yönetmeliği 39/1 (docs/adr/8083).

Bütün yapılarda kapı yüksekliği 2,10 m'den, net (temiz) genişliği bina giriş kapısında 1,50 m'den, bağımsız bölüm giriş
kapısında 1,00 m'den, diğer mahallerin kapılarında 0,90 m'den az olamaz. Hâlleri (``data/surumler.json``):

- **1.10.2017** (RG 3.7.2017/30113; b bendi RG 30.9.2017/30196 ile, yürürlükten önce): balkon ve tuvalet kapıları
  0,80 m'ye düşürülebilir.
- **13.8.2025** (RG 32985, 6 ncı madde): balkon ve tuvalet kapılarının 0,80'i yok; d) yaşlılar için bina giriş kapısı
  cepheden ayırt edilir (çizimde okunmaz). Değişiklik yayımı günü yürürlüğe girdi; geçiş hükmü (Geçici 3/9) 39/1'i
  saymaz: başvuru gününde yürürlükteki hâl okunur.
- **Sivas 39/1 (10.11.2022):** PAİY'nin 2017 hâli gibi, balkon ve tuvalet kapıları 0,80 m.

**Kapının ne olduğu** (olgular ``building.doors``: net genişlik, yükseklik, iki yanında ne olduğu, tuvalet ya da
balkon kapısı olup olmadığı; ``units``, ``commons``):

- Bir yanında hiçbir şey çizilmemiş ya da programın dış alanı (bahçe, otopark) olan kapı dışarı açılır. Binada ortak
  alan ya da birden çok bağımsız bölüm varsa ortak alanın ya da grupsuz mahalin dış kapısı bina girişidir (b), bağımsız
  bölümün mahalininki o bölümün girişidir (c; zemin kattaki dairenin bahçe kapısı); eklentininki her binada diğer mahal
  kapısıdır. Binada ortak alan yoksa ve tek bölüm varsa dış kapı bölümün girişidir (müstakil evde bina girişi ile bölüm
  girişi aynı kapıdır, bölümünki okunur).
- Bir bağımsız bölümün mahali ile ortak alanın mahali arasındaki kapı bağımsız bölüm girişidir (c).
- Öbürleri diğer mahallerin kapısıdır (c); bir yanı tuvalet ya da balkon olan kapı, hâli izin veriyorsa 0,80 m. Balkonu
  mahal olarak çizilmemiş balkon kapısı dışarı açılır: balkon mahalini çizmek okumayı düzeltir.
- Sabit kapı ve hiçbir yanında mahal olmayan kapı okunmaz; üst kotu olmayan kapının yüksekliği okunmaz.
- **Çift kanatlı bina girişi** (b, docs/adr/8093): bir kanadın net genişliği 1,00 m'den az olamaz: ilk kanat geniş ya da
  eşitse onun tek başına net geçişi (`clearFirst`) okunur; ikinci kanat genişse tek başına net geçişi ölçülmez, uzman
  incelemesi.
- **Döner kapı** (ç): ancak 39/1'in ölçülerinde normal bir kapının yanında ilave olabilir: döner bina giriş kapısının
  kendi net genişliği okunmaz, aynı mahale açılan döner olmayan bir bina giriş kapısı aranır; yoksa kaldı.

**Gün yoksa** her kapı ayrı denenir (``yururluk.parcalarla``): sonucu güne göre değişmeyen kapının bulgusu en yeni hâlle
verilir, değişeninki o kapının "eksik veri" bulgusudur.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import katman
from . import yururluk as y

ULUSAL = [("PAİY", "39/1")]
YEREL = [("Sivas", "39/1")]
KONU = "kapı kuralı"
GENISLET = "Kapıyı genişletin ya da daha geniş bir kapı tipi seçin."
KANAT = "Kanatlardan birini en az 1,00 m net geçiş verecek kadar genişletin (kanat payı) ya da daha geniş bir kapı seçin."
DONER = "Döner kapının yanına 39/1'in ölçülerinde (net en az 1,50 m) normal bir bina giriş kapısı çizin."
YUKSELT = "Kapının yüksekliğini en az 2,10 m yapın."


def paylasimli(bina: dict[str, Any]) -> bool:
    """Binanın bölümlerinden ayrı bir girişi var mı: ortak alanı ya da birden çok bağımsız bölümü."""
    return int(bina.get("commons") or 0) > 0 or int(bina.get("units") or 0) > 1


#: Bir yanın binanın içindeki mahal olduğu değerleri (boşluk, dış alan ya da hiçbir şey değil).
MAHAL = ("unit", "common", "annex", "room")
#: Bir yanın dışarısı olduğu değerler: hiçbir şey çizilmemiş ya da programın dış alanı (bahçe, otopark).
DIS = (None, "outdoor")


def yeri(kapi: dict[str, Any], ortak: bool) -> str | None:
    """Kapının 39/1'deki yeri: ``bina`` (bina girişi), ``bolum`` (bağımsız bölüm girişi), ``diger``; hiçbir yanında
    binanın mahali yoksa okunmaz."""
    a, b = (list(kapi.get("sides") or []) + [None, None])[:2]
    if a not in MAHAL and b not in MAHAL:
        return None
    if (a in DIS or b in DIS) and not kapi.get("balcony"):
        ic = b if a in DIS else a
        if ic == "annex":
            return "diger"
        return "bolum" if ic == "unit" or not ortak else "bina"
    if {a, b} == {"unit", "common"}:
        return "bolum"
    return "diger"


def _genislik(hal: y.Hal, yer: str, kapi: dict[str, Any]) -> tuple[katman.Sinir, str]:
    """Hâlin kapıya koyduğu en az net genişlik ve iletideki öznesi."""
    a = hal.anahtar
    if yer == "bina":
        return katman.Sinir(float(a["building"]), hal, "b"), "Bina giriş kapısının net genişliği"
    if yer == "bolum":
        return katman.Sinir(float(a["unit"]), hal, "c"), "Bağımsız bölüm giriş kapısının net genişliği"
    if kapi.get("toilet") or kapi.get("balcony"):
        ozne = "Tuvalet kapısının net genişliği" if kapi.get("toilet") else "Balkon kapısının net genişliği"
        az = a.get("reduced")
        return katman.Sinir(float(az if az is not None else a["other"]), hal, "c"), ozne
    return katman.Sinir(float(a["other"]), hal, "c"), "Kapının net genişliği"


def _satir(hal: y.Hal, durum: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": ""}


def _kapi(kapi: dict[str, Any], ortak: bool, sivas: bool, gun: date, kapilar: list[dict[str, Any]]) -> y.Sonuc:
    """Bir kapının ``gun``deki bulguları ve maddelerin bu kapıdaki durumu; ``kapilar`` binanın okunan kapıları (döner
    kapının yanındaki normal kapı için)."""
    hu = y.hal("PAİY", "39/1", gun)
    hy = y.hal("Sivas", "39/1", gun) if sivas else None
    if hu is None:
        return y.Sonuc([], [])
    uids = [kapi["uid"]]
    bulgular: list[dict[str, Any]] = []
    durum_u, durum_y = y.GECTI, (y.GECTI if hy is not None else None)

    def topla(du: str, dy: str | None) -> None:
        nonlocal durum_u, durum_y
        if du == y.KALDI:
            durum_u = y.KALDI
        if dy == y.KALDI:
            durum_y = y.KALDI

    h = kapi.get("height")
    if h is not None:
        ulusal = katman.Sinir(float(hu.anahtar["height"]), hu, "a")
        yerel = katman.Sinir(float(hy.anahtar["height"]), hy, "a") if hy is not None else None
        b = katman.bulgu("paiy_door_height_below", "Kapı yüksekliği", float(h), True, ulusal, yerel, YUKSELT, uids,
                         f"{kapi['uid']}:yükseklik")
        if b:
            bulgular.append(b)
        topla(*katman.durumlar(float(h), True, ulusal, yerel))
    yer = yeri(kapi, ortak)
    v = kapi.get("clear")
    doner = kapi.get("operation") == "revolving"
    if yer is not None and v is not None and not (doner and yer == "bina"):
        ulusal, ozne = _genislik(hu, yer, kapi)
        yerel = _genislik(hy, yer, kapi)[0] if hy is not None else None
        b = katman.bulgu("paiy_door_width_below", ozne, float(v), True, ulusal, yerel, GENISLET, uids,
                         f"{kapi['uid']}:genişlik")
        if b:
            bulgular.append(b)
        topla(*katman.durumlar(float(v), True, ulusal, yerel))
    kanatlar = list(kapi.get("leaves") or [])
    if yer == "bina" and len(kanatlar) == 2 and not doner:
        # 39/1-b: çift kanatlı bina girişinde bir kanat en az 1,00 m (docs/adr/8093).
        ilk = kapi.get("clearFirst")
        if kanatlar[0] + 1e-9 >= kanatlar[1] and ilk is not None:
            ulusal = katman.Sinir(float(hu.anahtar["leaf"]), hu, "b")
            yerel = katman.Sinir(float(hy.anahtar["leaf"]), hy, "b") if hy is not None else None
            b = katman.bulgu("paiy_door_leaf_below", "Çift kanatlı bina giriş kapısının bir kanadının net genişliği",
                             float(ilk), True, ulusal, yerel, KANAT, uids, f"{kapi['uid']}:kanat")
            if b:
                bulgular.append(b)
            topla(*katman.durumlar(float(ilk), True, ulusal, yerel))
        else:
            bulgular.append({"code": "paiy_door_leaf_below", "state": y.UZMAN, "uids": uids, "repair": KANAT,
                             "message": f"Çift kanatlı bina giriş kapısının ikinci kanadı ({kanatlar[1]:.2f} m) ilkinden "
                                        f"({kanatlar[0]:.2f} m) geniş: tek başına net geçişi ölçülmüyor; 39/1-b bir "
                                        f"kanadın en az {float(hu.anahtar['leaf']):.2f} m olmasını ister — {hu.kisa('b')}",
                             "rule": hu.kural("b"), "_iz": f"{kapi['uid']}:kanat"})
            if durum_u == y.GECTI:
                durum_u = y.UZMAN
    if doner and yer == "bina":
        # 39/1-ç: döner kapı ancak normal bir kapının yanında ilave; yanı: aynı mahale açılan bina girişi.
        icerisi = {u for u in kapi.get("between") or [] if u}
        yaninda = any(k["uid"] != kapi["uid"] and k.get("operation") != "revolving" and yeri(k, ortak) == "bina"
                      and icerisi & {u for u in k.get("between") or [] if u} for k in kapilar)
        if not yaninda:
            bulgular.append({"code": "paiy_revolving_door_alone", "uids": uids, "repair": DONER,
                             "message": f"Döner bina giriş kapısının yanında normal bir bina giriş kapısı yok: 39/1-ç "
                                        f"döner kapıya ancak 39/1'in ölçülerinde normal bir kapının yanında ilave olarak "
                                        f"izin verir — {hu.kisa('ç')}", "rule": hu.kural("ç"), "_iz": f"{kapi['uid']}:döner"})
            durum_u = y.KALDI
            if hy is not None:
                durum_y = y.KALDI
    satirlar = [_satir(hu, durum_u)] + ([_satir(hy, durum_y)] if hy is not None and durum_y is not None else [])
    return y.Sonuc(bulgular, satirlar)


def okunur(kapi: dict[str, Any], ortak: bool) -> bool:
    """Kuralın okuduğu kapı: yüksekliği ya da yeri ve net genişliği belli."""
    return kapi.get("height") is not None or (yeri(kapi, ortak) is not None and kapi.get("clear") is not None)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin kapı yüksekliği ve net genişliği bulguları: her kapı başvuru gününün hâlleriyle; gün yoksa her kapı
    her hâlle denenmiş."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    sivas = bina.get("local") == "sivas"
    maddeler = ULUSAL + (YEREL if sivas else [])
    ortak = paylasimli(bina)
    kapilar = [k for k in bina.get("doors") or [] if okunur(k, ortak)]
    if not kapilar:
        hallar = [y.hal(yon, m, gun) if gun is not None else y.haller(yon, m)[-1] for yon, m in maddeler]
        return y.Sonuc([], [dict(_satir(h, y.KAPSAM_DISI), note="çizimde ölçüsü okunan kapı yok") for h in hallar
                            if h is not None])
    if gun is not None and y.hal("PAİY", "39/1", gun) is None:
        ilk = y.haller("PAİY", "39/1")[0]
        ileti = (f"Başvuru günü ({y.yaz(gun)}) PAİY 39/1'in kural setindeki ilk hâlinden ({ilk.etiket}) önce: o günün "
                 f"metni KentOS'ta yok")
        bulgu = {"code": "regulation_text_unknown", "message": ileti,
                 "repair": "Kapıları maddenin başvuru günündeki metnine göre uzman incelesin.",
                 "uids": [k["uid"] for k in kapilar][:20], "rule": "PAİY 39/1", "_iz": "39/1:bilinmiyor",
                 "state": y.UZMAN}
        return y.Sonuc([bulgu], [{"rule": "PAİY 39/1", "title": ilk.baslik, "version": None, "gazette": None,
                                  "state": y.UZMAN, "note": ileti}])
    sonuclar = y.parcalarla(kapilar, lambda k, g: _kapi(k, ortak, sivas, g, kapilar), maddeler, gun, neden, KONU,
                            lambda k: [k["uid"]])
    bulgular = [b for s in sonuclar for b in s.bulgular]
    satirlar = []
    for r in y.birlesik(sonuclar):
        sayi = r.pop("sayilar")
        if r["state"] == y.EKSIK_VERI:
            r["note"] = f"{sayi.get(y.EKSIK_VERI, 0)} kapının sonucu başvuru gününe bağlı"
        elif r["state"] == y.KALDI:
            r["note"] = f"{sayi.get(y.KALDI, 0)} kapı yetmiyor ({len(kapilar)} kapı okundu)"
        else:
            r["note"] = f"{len(kapilar)} kapı yetiyor"
        satirlar.append(r)
    return y.Sonuc(bulgular, satirlar)
