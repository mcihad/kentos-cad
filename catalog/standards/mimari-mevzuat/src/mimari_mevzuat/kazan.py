"""Kazan dairesi ve yakıt deposu: Binaların Yangından Korunması Hakkında Yönetmelik 54/2, 54/4, 54/5 ve 56/1
(docs/adr/8113; plan/bim-plan.md BIM-13'ün yangın kuralları).

- **Kazan dairesi** adı Kazan (Kazan dairesi, Kazan odası), Kalorifer ya da Isı merkezi olan mahaldir.
- **54/2 (19.12.2007):** kazan dairesi binanın öbür kısımlarından en az 120 dakika yangına dayanıklı bölmelerle ayrılır.
  Her komşu mahalle ayrımı okunur: duvarlar ve pencereler en az EI 120 beyan edilmiş, duvarsız (açık) yer ve eşiksiz
  boşluk yok; kapılar 54/5'in işi. Beyansız eleman eksik veri. Döşemeler ve dış duvarlar olgularda değil; "merkezi bir
  yerde ve bütün hâlinde", bina dilatasyonu çizimden okunmaz (satırın notu).
- **54/4 (9.9.2009):** kazan dairesinin kapısı kaçış merdivenine ya da genel kullanım merdivenine doğrudan açılmaz, bir
  ortak hol ya da koridora açılır. Merdiven mahaline açılan kapı ve hol ya da koridor olmayan mahale açılan kapı kaldı;
  dışarı açılan kapı uzman incelemesi (fıkra hol ya da koridor der). Holün ortak olup olmadığı olgularda değil.
- **54/5 (19.12.2007):** döşeme alanı (net, iç yüzlerden) 100 m²'nin üzerinde ya da ısıl kapasitesi 350 kW'ın üzerinde olan
  kazan dairesinde en az 2 çıkış kapısı, olabildiğince ters yönde; kapılar en az 90 dakika, duman sızdırmaz,
  kendiliğinden kapanır. Isıl kapasite çizimde yok: 100 m²'yi aşmayan tek kapılı kazan dairesi uzman incelemesi (tesisat
  projesinin kapasitesi); kapıların yönü ölçü değil, idarenin.
- **56/1 (19.12.2007):** yakıt deposu (Yakıt, Akaryakıt, Mazot adlı mahal) ile kazan dairesi arasında en az 120 dakika
  dayanıklı bölme: aralarındaki ayrım 54/2 gibi okunur, kapısı da en az 120 dakika.

**Okunan olgular** (``facts.building``): mahaller (adı, katı, net alanı, kaçış merdiveni mi), iki mahal arasındaki
ayrımlar (duvarlar, açıklıklar, beyan edilen sınıfları, açık boy), kapılar (iki yanı, beyan edilen sınıfı, merdivene
açılıp açılmadığı).
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import yururluk as y
from .kullanim import _koridor, _sozcukler
from .yangin_sinifi import sinif

MADDELER = [("BYKHY", "54/2"), ("BYKHY", "54/4"), ("BYKHY", "54/5"), ("BYKHY", "56/1")]

#: Bir açık yer bundan kısaysa sayılmaz (çizimin köşe gürültüsü), m (kompartıman kuralının eşiği).
_EN_AZ = 0.05


def kazan_dairesi(ad: str | None) -> bool:
    """Adı kazan dairesi diyen mahal: Kazan…, Kalorifer…, Isı merkezi."""
    w = _sozcukler(ad)
    if any(x.startswith("KAZAN") or x.startswith("KALORIFER") for x in w):
        return True
    return any(a == "ISI" and b.startswith("MERKEZ") for a, b in zip(w, w[1:]))


def yakit_deposu(ad: str | None) -> bool:
    """Adı yakıt deposu diyen mahal: Yakıt…, Akaryakıt…, Mazot…, Kömürlük."""
    w = _sozcukler(ad)
    return any(x.startswith(("YAKIT", "AKARYAKIT", "MAZOT", "KOMUR")) for x in w)


def _m2(x: float) -> str:
    return f"{x:.2f}".replace(".", ",")


def _m(x: float) -> str:
    return f"{x:.2f}".replace(".", ",")


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


def _durum(kalan: int, eksik: int, uzman: int, gecti: str, kaldi: str, eksik_not: str,
           uzman_not: str = "") -> tuple[str, str]:
    if kalan:
        return y.KALDI, kaldi
    if eksik:
        return y.EKSIK_VERI, eksik_not
    if uzman:
        return y.UZMAN, uzman_not
    return y.GECTI, gecti


def _bolme(bina: dict[str, Any], hal: y.Hal, code: str, kim: str, oteki: str, a: str, b: str, ad_a: str, ad_b: str,
           kapilar_da: bool) -> tuple[list[dict[str, Any]], int, int]:
    """İki mahal arasındaki ayrımın en az 120 dakika (EI) olup olmadığı: bulgular, kalan ve beyansız sayısı."""
    dakika = int(hal.anahtar["minutes"])
    bulgular: list[dict[str, Any]] = []
    kalan = eksik = 0
    for ayrim in bina.get("separations") or []:
        if sorted(ayrim.get("rooms") or []) != sorted([a, b]):
            continue
        acik = float(ayrim.get("open") or 0.0)
        if acik > _EN_AZ:
            kalan += 1
            bulgular.append(_bulgu(
                code, f"“{ad_a}” ile “{ad_b}” arasında duvarsız {_m(acik)} m: {kim} {oteki} en az {dakika} dakika "
                "dayanıklı bölmeyle ayrılır", f"Aralarına en az EI {dakika} duvar yapın ve beyan edin.", hal, [a, b],
                f"{a}:{b}:açık"))
        for o in list(ayrim.get("walls") or []) + list(ayrim.get("openings") or []):
            tur = o.get("kind") or "wall"
            if tur == "door" and not kapilar_da:
                continue
            ne = {"wall": "duvar", "window": "pencere", "door": "kapı", "gap": "boşluk"}.get(tur, tur)
            if tur == "gap":
                kalan += 1
                bulgular.append(_bulgu(
                    code, f"“{ad_a}” ile “{ad_b}” arasında eşiksiz boşluk: {kim} {oteki} en az {dakika} dakika dayanıklı "
                    "bölmeyle ayrılır", f"Boşluğu kapatın ya da en az EI {dakika} kapı koyun.", hal, [o["uid"], a, b],
                    f"{o['uid']}:boşluk"))
                continue
            beyan = o.get("fireRating")
            okunan = sinif(beyan)
            if okunan is None:
                eksik += 1
                neden = f"“{beyan}” okunamadı" if beyan else "yangın sınıfı beyan edilmemiş"
                bulgular.append(_bulgu(
                    code, f"“{ad_a}” ile “{ad_b}” arasındaki {ne}: {neden}; en az {dakika} dakika gerekir",
                    f"Elemanın (ya da tipinin) yangın sınıfını beyan edin (EI {dakika}).", hal, [o["uid"], a, b],
                    f"{o['uid']}:beyan", y.EKSIK_VERI))
            elif okunan["dakika"] < dakika or "I" not in okunan["harf"]:
                kalan += 1
                bulgular.append(_bulgu(
                    code, f"“{ad_a}” ile “{ad_b}” arasındaki {ne} “{beyan}”: en az EI {dakika} gerekir",
                    f"Elemanı en az EI {dakika} yapın ve beyan edin.", hal, [o["uid"], a, b], f"{o['uid']}:sınıf"))
    return bulgular, kalan, eksik


def ayrimlar(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """54/2: kazan dairesinin komşu mahallerden en az 120 dakika bölmelerle ayrılması."""
    mahaller = {s["uid"]: s for s in bina.get("spaces") or []}
    kazanlar = [u for u, s in mahaller.items() if kazan_dairesi(s.get("name"))]
    if not kazanlar:
        return [], _satir(hal, y.KAPSAM_DISI, "kazan dairesi yok")
    bulgular: list[dict[str, Any]] = []
    kalan = eksik = komsu = 0
    for k in kazanlar:
        for ayrim in bina.get("separations") or []:
            iki = ayrim.get("rooms") or []
            if k not in iki:
                continue
            o = next((u for u in iki if u != k), None)
            if o is None or o in kazanlar or yakit_deposu((mahaller.get(o) or {}).get("name")):
                continue  # Yakıt deposuyla ayrım 56/1'in.
            komsu += 1
            b, n_k, n_e = _bolme(bina, hal, "bykhy_boiler_room_separation", "kazan dairesi", "binanın öbür kısımlarından",
                                 k, o, mahaller[k].get("name") or "Kazan dairesi", mahaller[o].get("name") or "Mahal",
                                 False)
            bulgular += b
            kalan += n_k
            eksik += n_e
    if not komsu:
        return [], _satir(hal, y.KAPSAM_DISI, "kazan dairesinin bitişik mahali yok")
    durum, not_ = _durum(kalan, eksik, 0, f"{len(kazanlar)} kazan dairesinin {komsu} komşusuyla ayrımı en az EI 120 "
                         "(döşemeler, merkezi yeri ve dilatasyon çizimden okunmaz)",
                         f"{kalan} ayrım yetmiyor", f"{eksik} elemanın sınıfı beyan edilmemiş ya da okunamadı")
    return bulgular, _satir(hal, durum, not_)


def _kapilar(bina: dict[str, Any], uid: str) -> list[dict[str, Any]]:
    """Kazan dairesinin kapıları: kendi yakıt deposuna ya da kömürlüğüne açılan kapı onun çıkış kapısı değildir (56/1)."""
    mahaller = {s["uid"]: s for s in bina.get("spaces") or []}
    out = []
    for k in bina.get("doors") or []:
        iki = k.get("between") or []
        if uid not in iki:
            continue
        o = next((u for u in iki if u != uid), None)
        if o is not None and yakit_deposu((mahaller.get(o) or {}).get("name")):
            continue
        out.append(k)
    return out


def kapilarin_yonu(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """54/4: kazan dairesinin kapısı merdivene değil, ortak hol ya da koridora açılır."""
    mahaller = {s["uid"]: s for s in bina.get("spaces") or []}
    kazanlar = [u for u, s in mahaller.items() if kazan_dairesi(s.get("name"))]
    if not kazanlar:
        return [], _satir(hal, y.KAPSAM_DISI, "kazan dairesi yok")
    kapilar = [(k, kapi) for k in kazanlar for kapi in _kapilar(bina, k)]
    kapisiz = [k for k in kazanlar if not _kapilar(bina, k)]
    if not kapilar:
        return [_bulgu("bykhy_boiler_room_door", f"“{mahaller[k].get('name') or 'Kazan dairesi'}”: kapısı çizilmemiş",
                       "Kazan dairesinin kapısını bir ortak hol ya da koridora açılacak biçimde yerleştirin.", hal, [k],
                       f"{k}:kapı", y.EKSIK_VERI) for k in kapisiz], _satir(hal, y.EKSIK_VERI,
                                                                           "kazan dairesinin kapısı çizilmemiş")
    bulgular: list[dict[str, Any]] = []
    kalan = uzman = 0
    for k, kapi in kapilar:
        ad = mahaller[k].get("name") or "Kazan dairesi"
        o = next((u for u in kapi.get("between") or [] if u != k), None)
        oteki = mahaller.get(o) if o else None
        if kapi.get("stair") or (oteki is not None and oteki.get("stair")):
            kalan += 1
            bulgular.append(_bulgu(
                "bykhy_boiler_room_door", f"“{ad}” kapısı merdivene açılıyor (“{(oteki or {}).get('name') or 'Merdiven'}”)"
                ": kaçış ya da genel kullanım merdivenine doğrudan açılamaz", "Kapıyı bir ortak hol ya da koridora "
                "açın.", hal, [kapi["uid"], k], f"{kapi['uid']}:merdiven"))
        elif oteki is None:
            uzman += 1
            bulgular.append(_bulgu(
                "bykhy_boiler_room_door", f"“{ad}” kapısı dışarı açılıyor: 54/4 ortak hol ya da koridora açılmasını "
                "söyler, dışarı açılan kapıyı anmaz", "Kapının yerini belgeleyin; idare karar verir.", hal,
                [kapi["uid"], k], f"{kapi['uid']}:dışarı", y.UZMAN))
        elif not _koridor(oteki.get("name")):
            kalan += 1
            bulgular.append(_bulgu(
                "bykhy_boiler_room_door", f"“{ad}” kapısı “{oteki.get('name') or 'mahal'}” mahaline açılıyor: bir ortak "
                "hol ya da koridora açılması gerekir", "Kapıyı bir ortak hol ya da koridora açın.", hal,
                [kapi["uid"], k, o], f"{kapi['uid']}:hol"))
    durum, not_ = _durum(kalan, 0, uzman, f"{len(kapilar)} kapı hol ya da koridora açılıyor (holün ortak olduğu "
                         "olgularda yok)", f"{kalan} kapı merdivene ya da hol ve koridor olmayan mahale açılıyor", "",
                         f"{uzman} kapı dışarı açılıyor")
    return bulgular, _satir(hal, durum, not_)


def cikislar(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """54/5: büyük kazan dairesinin iki çıkış kapısı ve kapıların sınıfı."""
    alan_siniri, dakika = float(hal.anahtar["area"]), int(hal.anahtar["minutes"])
    mahaller = {s["uid"]: s for s in bina.get("spaces") or []}
    kazanlar = [u for u, s in mahaller.items() if kazan_dairesi(s.get("name"))]
    if not kazanlar:
        return [], _satir(hal, y.KAPSAM_DISI, "kazan dairesi yok")
    bulgular: list[dict[str, Any]] = []
    kalan = eksik = uzman = 0
    for k in kazanlar:
        ad = mahaller[k].get("name") or "Kazan dairesi"
        kapilar = _kapilar(bina, k)
        net = mahaller[k].get("net")
        if net is None:
            eksik += 1
            bulgular.append(_bulgu(
                "bykhy_boiler_room_exits", f"“{ad}”: net alanı ölçülemedi (sınırı kaynaklarından bulunamıyor)",
                "Mahalin sınırını duvarlarına göre yeniden bulun ya da çizin.", hal, [k], f"{k}:alan", y.EKSIK_VERI))
        elif not kapilar:
            eksik += 1
            bulgular.append(_bulgu(
                "bykhy_boiler_room_exits", f"“{ad}”: kapısı çizilmemiş", "Kazan dairesinin çıkış kapısını yerleştirin.",
                hal, [k], f"{k}:kapı", y.EKSIK_VERI))
        elif float(net) > alan_siniri and len(kapilar) < 2:
            kalan += 1
            bulgular.append(_bulgu(
                "bykhy_boiler_room_exits", f"“{ad}”: döşeme alanı {_m2(float(net))} m² > 100 m², {len(kapilar)} kapı; en "
                "az 2 çıkış kapısı, olabildiğince ters yönde", "Kazan dairesine öbür kapının ters yönünde ikinci bir çıkış "
                "kapısı açın.", hal, [k] + [x["uid"] for x in kapilar], f"{k}:kapı"))
        elif len(kapilar) < 2:
            uzman += 1
            bulgular.append(_bulgu(
                "bykhy_boiler_room_exits", f"“{ad}”: {len(kapilar)} kapı; ısıl kapasitesi 350 kW'ı aşıyorsa en az 2 çıkış "
                "kapısı gerekir, kapasite çizimde yok", "Tesisat projesindeki ısıl kapasiteyi belgeleyin; 350 kW'ı "
                "aşıyorsa ikinci bir çıkış kapısı açın.", hal, [k] + [x["uid"] for x in kapilar], f"{k}:kapasite",
                y.UZMAN))
        for kapi in kapilar:
            beyan = kapi.get("fireRating")
            okunan = sinif(beyan)
            if okunan is None:
                eksik += 1
                neden = f"“{beyan}” okunamadı" if beyan else "yangın sınıfı beyan edilmemiş"
                bulgular.append(_bulgu(
                    "bykhy_boiler_room_door_rating", f"“{ad}” kapısının {neden}; en az {dakika} dakika, duman sızdırmaz "
                    "ve kendiliğinden kapanan kapı gerekir", f"Kapının (ya da tipinin) yangın sınıfını beyan edin "
                    f"(EI2 {dakika}-S200C5).", hal, [kapi["uid"], k], f"{kapi['uid']}:beyan", y.EKSIK_VERI))
                continue
            eksikler = []
            if okunan["dakika"] < dakika:
                eksikler.append(f"{okunan['dakika']} dakika < {dakika} dakika")
            if not okunan["duman"]:
                eksikler.append("duman sızdırmaz değil (S yok)")
            if not okunan["kapanma"]:
                eksikler.append("kendiliğinden kapanmıyor (C yok)")
            if eksikler:
                kalan += 1
                bulgular.append(_bulgu(
                    "bykhy_boiler_room_door_rating", f"“{ad}” kapısı “{beyan}”: " + "; ".join(eksikler),
                    f"Kapıyı en az {dakika} dakika, duman sızdırmaz ve kendiliğinden kapanan yapın, beyan edin "
                    f"(EI2 {dakika}-S200C5).", hal, [kapi["uid"], k], f"{kapi['uid']}:sınıf"))
    durum, not_ = _durum(kalan, eksik, uzman, f"{len(kazanlar)} kazan dairesinin çıkışları ve kapıları yetiyor "
                         "(kapıların yönü idarenin)", f"{kalan} çıkış ya da kapı yetmiyor",
                         f"{eksik} kapı ya da alan okunamadı", f"{uzman} kazan dairesinin ısıl kapasitesi çizimde yok")
    return bulgular, _satir(hal, durum, not_)


def yakit(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """56/1: yakıt deposu ile kazan dairesi arasında en az 120 dakika dayanıklı bölme (kapısı da)."""
    mahaller = {s["uid"]: s for s in bina.get("spaces") or []}
    kazanlar = {u for u, s in mahaller.items() if kazan_dairesi(s.get("name"))}
    depolar = {u for u, s in mahaller.items() if yakit_deposu(s.get("name")) and u not in kazanlar}
    ciftler = sorted({tuple(sorted(a.get("rooms") or [])) for a in bina.get("separations") or []
                      if len(set(a.get("rooms") or []) & kazanlar) == 1 and len(set(a.get("rooms") or []) & depolar) == 1})
    if not ciftler:
        return [], _satir(hal, y.KAPSAM_DISI, "kazan dairesine bitişik yakıt deposu yok")
    bulgular: list[dict[str, Any]] = []
    kalan = eksik = 0
    for a, b in ciftler:
        k, d = (a, b) if a in kazanlar else (b, a)
        bl, n_k, n_e = _bolme(bina, hal, "bykhy_fuel_store_separation", "yakıt deposu", "kazan dairesinden", d, k,
                              mahaller[d].get("name") or "Yakıt deposu", mahaller[k].get("name") or "Kazan dairesi", True)
        bulgular += bl
        kalan += n_k
        eksik += n_e
    durum, not_ = _durum(kalan, eksik, 0, f"{len(ciftler)} yakıt deposu kazan dairesinden en az EI 120 ayrılmış "
                         "(havalandırma ve havuzlama çizimden okunmaz)", f"{kalan} eleman yetmiyor",
                         f"{eksik} elemanın sınıfı beyan edilmemiş ya da okunamadı")
    return bulgular, _satir(hal, durum, not_)


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin kazan dairesi ve yakıt deposu bulguları."""
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    for madde, denetim in (("54/2", ayrimlar), ("54/4", kapilarin_yonu), ("54/5", cikislar), ("56/1", yakit)):
        hal = y.hal("BYKHY", madde, gun)
        if hal is None:
            ilk = y.haller("BYKHY", madde)[0]
            satirlar.append({"rule": f"BYKHY {madde}", "title": ilk.baslik, "version": None, "gazette": None,
                             "state": y.UZMAN,
                             "note": f"başvuru günü ({y.yaz(gun)}) kural setinin bildiği ilk hâlden ({ilk.etiket}) önce"})
            continue
        b, s = denetim(bina, hal)
        bulgular += b
        satirlar.append(s)
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin kazan dairesi ve yakıt deposu bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "kazan dairesi kuralı")
