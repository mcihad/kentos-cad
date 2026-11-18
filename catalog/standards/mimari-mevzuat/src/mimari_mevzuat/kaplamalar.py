"""Kaplamaların yanıcılığı: Binaların Yangından Korunması Hakkında Yönetmelik 20/3, 26/2, 26/4 ve Ek-2/Ç (docs/adr/8091,
8138).

- **20/3 (9.7.2015):** duvarlarda iç kaplamalar en az normal alevlenici; yüksek binada ve kapasitesi 100 kişiden fazla
  sinema, tiyatro, konferans ve düğün salonu gibi yerlerde en az zor alevlenici. Mahallerin duvar kaplamasından
  (docs/adr/8138); 100 kişiyi aşan başka adlı salonda normal alevlenici kaplama uzman incelemesi ("gibi yerler"). İçte
  uygulanan ısı ve ses yalıtımı okunmaz.

- **26/2:** döşeme kaplamaları (IFC türü FLOORING) en az normal alevlenici, yüksek binada (4/ccc: bina yüksekliği 21,50
  m'den ya da yapı yüksekliği 30,50 m'den fazla) en az zor alevlenici malzemeden.
- **26/4:** ayrık nizamda müstakil konutlar dışındaki binaların tavan kaplamaları ve asma tavanları (IFC türü CEILING) en
  az zor alevlenici malzemeden. Binanın nizamı çizimde yok: tek konutlu binada (başka bölümü olmayan) tutmayan tavan
  uzman incelemesidir.
- **Ek-2/Ç:** malzemenin beyan edilen TS EN 13501-1 sınıfı (`materials[].reaction`) yanıcılık özelliğine çevrilir: hiç
  yanmaz, zor yanıcı, zor alevlenici, normal alevlenici, kolay alevlenici. Döşeme kaplamasının sınıfı döşeme tablosundan
  (``fl``), öbürlerininki öbür tablodan; sınıf önce tam yazımıyla, yoksa harfiyle eşlenir (harfin en zayıf satırı).

Mahallerin kaplamaları (docs/adr/8138): mahal listesinin yazdığı döşeme, duvar ve tavan kaplaması adı projenin aynı
adlı (ya da kimlikli) malzemesidir; aynı adlı kaplamalar tek bulguda, mahalleriyle. Döşemeninki 26/2'nin, tavanınki
26/4'ün, duvarınki 20/3'ün kaplamasıdır.

Malzemesi yoksa, malzemenin sınıfı beyan edilmemişse ya da öbür tablonun sınıfıysa eksik veri; yetmiyorsa kaldı.

**Okunan olgular** (``facts.building``): kaplamalar (``coverings``: tür, kat, malzeme, malzemenin sınıfı), mahallerin
kaplamaları (``spaces[].finishes``: kısım, ad, malzeme, sınıfı), mahallerin kişi yükü (``rooms``), katlar, bina ve yapı
yüksekliği, konut ve bölüm sayısı.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import yururluk as y

MADDELER = [("BYKHY", "20/3"), ("BYKHY", "26/2"), ("BYKHY", "26/4")]

#: Mahal kaplamasının kısmı, kaplamanın IFC türüne göre (docs/adr/8138).
_KISIM = {"FLOORING": "floor", "CEILING": "ceiling", "WALL": "wall"}


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def ozellik(sinif: str, doseme: bool, tablo: dict[str, Any]) -> tuple[str | None, str | None]:
    """Ek-2/Ç'ye göre ``sinif``'ın yanıcılık özelliği (anahtarı) ve okunmuyorsa nedeni; ``doseme`` döşeme tablosu."""
    satirlar = tablo["floor" if doseme else "other"]
    fl = "fl" in sinif
    if fl != doseme:
        return None, (f"“{sinif}” {'döşeme' if fl else 'döşeme dışı'} malzemesinin sınıfı; {'döşeme' if doseme else 'döşeme dışı'} "
                      f"kaplaması için Ek-2/Ç'nin {'döşeme' if doseme else 'öbür'} tablosunun sınıfı gerekir")
    tam = next((k for s, k in satirlar if s == sinif), None)
    if tam:
        return tam, None
    harf = sinif.split("-")[0]
    kok = next((k for s, k in satirlar if s == harf), None)
    if kok:
        return kok, None
    return None, f"“{sinif}” Ek-2/Ç'nin tablosunda yok"


def _ad(tablo: dict[str, Any], anahtar: str) -> str:
    return dict(tablo["categories"])[anahtar]


def _sira(tablo: dict[str, Any], anahtar: str) -> int:
    return [k for k, _ in tablo["categories"]].index(anahtar)


def _mahal_kaplamalari(bina: dict[str, Any], kisim: str) -> dict[str, list[dict[str, Any]]]:
    """Mahallerin ``kisim`` (floor, wall, ceiling) kaplamaları adlarına göre: her ad, mahalleri ve kaplamasıyla."""
    gruplar: dict[str, list[dict[str, Any]]] = {}
    for m in bina.get("spaces") or []:
        for f in m.get("finishes") or []:
            if f.get("part") == kisim:
                gruplar.setdefault(f["name"], []).append({"mahal": m, "kaplama": f})
    return gruplar


def _ogeler(bina: dict[str, Any], tur: str, ne: str) -> list[dict[str, Any]]:
    """Okunacak kaplamalar: ``tur`` kaplama nesneleri (FLOORING, CEILING) ve mahallerin aynı adlı kaplamaları
    (docs/adr/8138), her biri nesneleri, sözü, malzemesi ve sınıfıyla."""
    katlar = {k["level"]: k.get("name") or k["level"] for k in bina.get("storeys") or []}
    out = [{"uids": [c["uid"]], "soz": f"{katlar.get(c.get('level') or '', 'Katsız')}: {ne}", "ad": None,
            "material": c.get("material"), "reaction": c.get("reaction"), "iz": c["uid"]}
           for c in bina.get("coverings") or [] if c.get("predefined") == tur]
    for ad, hepsi in _mahal_kaplamalari(bina, _KISIM[tur]).items():
        k = hepsi[0]["kaplama"]
        out.append({"uids": sorted(h["mahal"]["uid"] for h in hepsi), "soz": f"“{ad}” {ne} ({len(hepsi)} mahal)",
                    "ad": ad, "material": k.get("material"), "reaction": k.get("reaction"), "iz": f"mahal:{ad}"})
    return out


def _bulgu(o: dict[str, Any], hal: y.Hal, tablo: dict[str, Any], doseme: bool, en_az: str, kod: str,
           uzman: str | None) -> dict[str, Any] | None:
    """Bir kaplamanın (``_ogeler``'in öğesi) en az ``en_az`` olmadığının bulgusu; tutuyorsa yok."""
    malzeme, sinif = o.get("material"), o.get("reaction")
    if not malzeme:
        neden = f"Malzemeler'de “{o['ad']}” adlı malzeme yok" if o["ad"] else "malzemesi yok"
        durum: str | None = y.EKSIK_VERI
    elif not sinif:
        neden, durum = f"malzemesinin (“{malzeme}”) yanıcılık sınıfı beyan edilmemiş", y.EKSIK_VERI
    else:
        k, sorun = ozellik(sinif, doseme, tablo)
        if k is None:
            neden, durum = sorun, y.EKSIK_VERI
        elif _sira(tablo, k) <= _sira(tablo, en_az):
            return None
        else:
            neden, durum = f"malzemesi “{malzeme}” {sinif}, {_ad(tablo, k)}", uzman and y.UZMAN
    istek = _ad(tablo, en_az)
    ileti = (f"{o['soz']}: {neden}; {hal.madde} en az {istek} ister"
             + (f" ({uzman})" if durum == y.UZMAN and uzman else ""))
    onarim = ("Kaplamanın malzemesini seçin ve malzemenin TS EN 13501-1 sınıfını beyan edin (Malzemeler › Yanıcılık "
              "sınıfı) ya da daha az yanıcı bir malzeme seçin.")
    if o["ad"]:
        onarim = (f"Malzemeler'de kaplamanın adıyla (“{o['ad']}”) bir malzeme açıp TS EN 13501-1 sınıfını beyan edin ya da "
                  f"mahallerde daha az yanıcı bir kaplama seçin.")
    b = {"code": kod, "message": f"{ileti} — {hal.kisa()}", "repair": onarim, "uids": o["uids"], "rule": hal.kural(),
         "_iz": f"{o['iz']}:{hal.madde}"}
    if durum:
        b["state"] = durum
    return b


def _denetle(bina: dict[str, Any], hal: y.Hal, tur: str, en_az: str, ne: str, kod: str, uzman: str | None
             ) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """``tur`` kaplamaların (FLOORING, CEILING) ve mahallerin aynı kısımdaki kaplamalarının en az ``en_az`` olup
    olmadığı."""
    tablo = hal.anahtar["ek2c"]
    hepsi = _ogeler(bina, tur, ne)
    if not hepsi:
        return [], _satir(hal, y.KAPSAM_DISI, f"{ne} yok")
    doseme = tur == "FLOORING"
    istek = _ad(tablo, en_az)
    bulgular = [b for o in hepsi if (b := _bulgu(o, hal, tablo, doseme, en_az, kod, uzman))]
    if any("state" not in b for b in bulgular):
        satir = _satir(hal, y.KALDI, f"{sum(1 for b in bulgular if 'state' not in b)} {ne} en az {istek} değil")
    elif any(b.get("state") == y.EKSIK_VERI for b in bulgular):
        satir = _satir(hal, y.EKSIK_VERI, f"{ne} malzemesinin yanıcılık sınıfı okunmuyor")
    elif bulgular:
        satir = _satir(hal, y.UZMAN, uzman or "")
    else:
        satir = _satir(hal, y.GECTI, f"{len(hepsi)} {ne} en az {istek}")
    return bulgular, satir


def _katla(metin: str) -> str:
    tablo = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                           "ş": "S", "Ş": "S", "ü": "U", "Ü": "U", "â": "A", "Â": "A", "î": "I", "Î": "I"})
    return metin.translate(tablo).upper()


def duvar(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """20/3 (docs/adr/8138): mahallerin duvar kaplamaları en az normal alevlenici; yüksek binada ve kapasitesi 100
    kişiden fazla sinema, tiyatro, konferans, düğün salonu gibi yerlerde en az zor alevlenici. Aynı adlı ve aynı isteği
    olan kaplamalar tek bulguda."""
    tablo = hal.anahtar["ek2c"]
    gruplar = _mahal_kaplamalari(bina, "wall")
    if not gruplar:
        return [], _satir(hal, y.KAPSAM_DISI, "mahallerde duvar kaplaması yazılmamış")
    h, s = float(bina.get("height") or 0), float(bina.get("structureHeight") or 0)
    yuksek = h > float(hal.anahtar["building"]) + 1e-9 or s > float(hal.anahtar["structure"]) + 1e-9
    kisi = {r["uid"]: float(r.get("persons") or 0) for r in bina.get("rooms") or []}
    esik, salonlar = float(hal.anahtar["persons"]), list(hal.anahtar["halls"])
    # Her ad, istenen sınıf ve "gibi yerler"in belirsizliğiyle: (ad, en az, neden, uzman sınırı).
    ogeler: dict[tuple[str, str, str, str | None], list[dict[str, Any]]] = {}
    for ad, hepsi in gruplar.items():
        for h_ in hepsi:
            m = h_["mahal"]
            kalabalik = kisi.get(m["uid"], 0) > esik + 1e-9
            salon = kalabalik and any(w in _katla(m.get("name") or "") for w in salonlar)
            if yuksek:
                anahtar = (ad, hal.anahtar["high"], "yüksek bina", None)
            elif salon:
                anahtar = (ad, hal.anahtar["high"], f"{esik:.0f} kişiyi aşan salon", None)
            elif kalabalik:
                # "Gibi yerler": adı sayılanlardan değil ama 100 kişiyi aşan mahal; normal alevlenici uzman incelemesi.
                anahtar = (ad, hal.anahtar["least"], f"{esik:.0f} kişiyi aşan mahal", hal.anahtar["high"])
            else:
                anahtar = (ad, hal.anahtar["least"], "", None)
            ogeler.setdefault(anahtar, []).append(h_)
    bulgular = []
    for (ad, en_az, neden, uzman_siniri), hepsi in ogeler.items():
        k = hepsi[0]["kaplama"]
        soz = f"“{ad}” duvar kaplaması ({len(hepsi)} mahal" + (f", {neden})" if neden else ")")
        o = {"uids": sorted(x["mahal"]["uid"] for x in hepsi), "soz": soz, "ad": ad, "material": k.get("material"),
             "reaction": k.get("reaction"), "iz": f"mahal:{ad}:{en_az}:{neden}"}
        b = _bulgu(o, hal, tablo, False, en_az, "bykhy_wall_lining_reaction", None)
        if b is None and uzman_siniri is not None:
            # 100 kişiyi aşan, adı sayılanlardan olmayan mahal: zor alevlenici değilse "gibi yerler"den mi uzman bakar.
            b = _bulgu(o, hal, tablo, False, uzman_siniri, "bykhy_wall_lining_reaction",
                       "100 kişiyi aşan mahal: sinema, tiyatro, konferans ya da düğün salonu gibi bir yer mi, uzman "
                       "karar verir")
            if b is not None and "state" not in b:
                b["state"] = y.UZMAN
        if b is not None:
            bulgular.append(b)
    istek = _ad(tablo, hal.anahtar["least"])
    if any("state" not in b for b in bulgular):
        satir = _satir(hal, y.KALDI, f"{sum(1 for b in bulgular if 'state' not in b)} duvar kaplaması yetmiyor")
    elif any(b.get("state") == y.EKSIK_VERI for b in bulgular):
        satir = _satir(hal, y.EKSIK_VERI, "duvar kaplamasının malzemesi ya da yanıcılık sınıfı okunmuyor")
    elif bulgular:
        satir = _satir(hal, y.UZMAN, "100 kişiyi aşan mahallerin duvar kaplaması uzman incelemesi")
    else:
        satir = _satir(hal, y.GECTI, f"{len(gruplar)} duvar kaplaması en az {istek}"
                                      + (" (yüksek bina: zor alevlenici)" if yuksek else ""))
    return bulgular, satir


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin kaplama bulguları."""
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    for madde in ("20/3", "26/2", "26/4"):
        hal = y.hal("BYKHY", madde, gun)
        tur = {"20/3": "WALL", "26/2": "FLOORING", "26/4": "CEILING"}[madde]
        if hal is None:
            uids = sorted(o for x in _ogeler(bina, tur, "") for o in x["uids"]) if tur != "WALL" else sorted(
                h["mahal"]["uid"] for hepsi in _mahal_kaplamalari(bina, "wall").values() for h in hepsi)
            if not uids:
                continue
            ilk = y.haller("BYKHY", madde)[0]
            ileti = (f"Başvuru günü ({y.yaz(gun)}) BYKHY {madde} maddesinin kural setindeki ilk hâlinden ({ilk.etiket}) "
                     f"önce: o günün Ek-2 tablosu KentOS'ta yok")
            bulgular.append({"code": "regulation_text_unknown", "message": ileti,
                             "repair": "Kaplamaları maddenin başvuru günündeki metnine göre uzman incelesin.",
                             "uids": uids[:20], "rule": f"BYKHY {madde}", "_iz": f"{madde}:bilinmiyor",
                             "state": y.UZMAN})
            satirlar.append({"rule": f"BYKHY {madde}", "title": ilk.baslik, "version": None, "gazette": None,
                             "state": y.UZMAN, "note": ileti})
            continue
        if madde == "20/3":
            b, r = duvar(bina, hal)
        elif madde == "26/2":
            h, s = float(bina.get("height") or 0), float(bina.get("structureHeight") or 0)
            yuksek = h > float(hal.anahtar["building"]) + 1e-9 or s > float(hal.anahtar["structure"]) + 1e-9
            en_az = hal.anahtar["high" if yuksek else "least"]
            b, r = _denetle(bina, hal, tur, en_az, "döşeme kaplaması", "bykhy_floor_covering_reaction", None)
        else:
            konut, bolum = int(bina.get("dwellings") or 0), int(bina.get("units") or 0)
            # Tek konutlu bina ayrık nizamda müstakil konut olabilir: nizam çizimde yok.
            uzman = ("tek konutlu bina: ayrık nizamda müstakil konutsa 26/4 uygulanmaz, binanın nizamı çizimde yok"
                     if konut == 1 and bolum == 1 else None)
            b, r = _denetle(bina, hal, tur, hal.anahtar["least"], "tavan kaplaması", "bykhy_ceiling_reaction", uzman)
        bulgular += b
        satirlar.append(r)
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin kaplama bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "kaplama kuralı")
