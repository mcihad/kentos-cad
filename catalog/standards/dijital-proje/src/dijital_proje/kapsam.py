"""Dijital teslim kapsamı (docs/adr/8063; plan/bim-plan.md BIM-02): bu projede hangi yükümlülük, hangi tarihten,
hangi maddeyle başlar. Satırlar: yönetmeliğin kapsamı, 2B çizim ve belgelerin PDF/A'sı (Md. 4/2–3), IFC modeli
(Md. 4/5), modelden üretilen çizimlerin PDF/A'sı (Md. 9/1) ve e-PYS'de teslim (e-PYS Md. 4).

- **Tarihler ve eşikler veridir:** KentOS'un kural kataloğundaki ``kapsam-2026.json``. KentOS çağırırken kendi
  kataloğunu verir; paketteki kopya, veri verilmediğinde (bulut işi, test) okunur ve bir test ikisini bir tutar.
- **Girdiler ruhsat künyesindendir** (``facts.project.permit``): kullanıcının yazdığı metin. Toplam yapı inşaat alanı
  ve bodrum dışındaki kat adedi boşsa çizimden ölçülen değer kullanılır. Boş başvuru tarihi henüz yapılmamış
  başvurudur: bugün ya da daha sonra yapılacak sayılır.
- **Tahmin yoktur:** girilmemiş ya da okunamayan bir girdi sonucu değiştirebiliyorsa sonuç "teyit gerekli"dir. Karar,
  eksik girdinin alabileceği her değer aralığı için yeniden verilir; hepsi aynı sonucu veriyorsa o sonuç yazılır,
  vermiyorsa teyit istenir.
- **Okuma kararları** (docs/adr/8063): "geçen" ve "fazla olan" kesin büyüktür (K23); "… tarihine kadar uygulanmaz"
  hükmü o günden başlar (K24); başvuru ile ihale tarihinden erkeni esas alınır, yürürlükten önceki başvuru kapsam
  dışıdır (K25); konut dışı binada 4/2–3 kendi başına hiç başlamaz, IFC ve Md. 9/1 1.9.2033'te başlar (K26); hangi
  belediyenin nüfusu ve nüfusun yılı idareye sorulur (K27); tadilat kapsam dışı, ilave teyit ister (K28).

Sonuç dört durumdan biridir: ``applies`` (uygulanır), ``not_applicable`` (uygulanmaz, maddesiyle), ``not_yet`` (henüz
başlamadı, tarihiyle) ve ``confirm`` (teyit gerekli, eksik girdileriyle).
"""

from __future__ import annotations

import json
import re
from datetime import date, timedelta
from importlib import resources
from typing import Any, Callable

#: Sonuçların durumları ve pencerenin yazdığı adları.
DURUMLAR = {
    "applies": "Uygulanır",
    "not_applicable": "Uygulanmaz",
    "not_yet": "Henüz başlamadı",
    "confirm": "Teyit gerekli",
}

#: Bir kararın keşfedeceği en çok yol: aşılırsa teyit istenir (girdilerin hepsi boşken bile birkaç yüz yol yeter).
EN_COK_YOL = 20_000

_UZAK = date(9999, 12, 31)


def veri() -> dict[str, Any]:
    """Paketteki ``kapsam-2026.json``: KentOS'un kural kataloğunun kopyası."""
    metin = resources.files(__package__).joinpath("data/kapsam-2026.json").read_text(encoding="utf-8")
    return json.loads(metin)


def karar(permit: dict[str, Any], today: str | date | None = None, data: dict[str, Any] | None = None) -> dict[str, Any]:
    """``permit``: olguların ``permit`` bölümü (``particulars``, ``constructionArea``, ``storeys``); ``today``: kararın
    günü (``YYYY-AA-GG``; verilmezse bugün); ``data``: tarihler ve eşikler (verilmezse paketteki kopya).

    Döner: ``rows`` (her yükümlülük için ``id``, ``title``, ``state``, ``words``, ``from``, ``until``, ``articles``,
    ``reason``, ``missing``), ``problems`` (okunamayan girdiler, nedeni ve çözümüyle), ``inputs`` (kararın okuduğu
    değerler: yazılan ya da çizimden ölçülen) ve ``today``."""
    cal = data if data is not None else veri()
    gun = _gun_degeri(today) if today is not None else date.today()
    if gun is None:
        raise ValueError(f"Kararın günü “{today}” okunamadı: YYYY-AA-GG yazın.")
    girdiler = {g["name"]: g for g in cal["inputs"]}
    bilinen, sorunlar, kaynak = _girdiler(permit or {}, girdiler)
    adaylar = _adaylar(cal, girdiler)
    satirlar = []
    for yukum in [{"id": "kapsam", "title": "Yönetmeliğin kapsamı"}, *cal["obligations"]]:
        karar_ver = _KARARLAR[yukum["id"]]
        satirlar.append(_kesfet(yukum, lambda d, k=karar_ver: k(d, cal, gun), bilinen, adaylar, girdiler))
    return {
        "today": gun.isoformat(),
        "rows": satirlar,
        "problems": sorunlar,
        "inputs": {ad: _goster(deger) for ad, deger in sorted(bilinen.items()) if deger is not None} | kaynak,
    }


# --- Girdiler ---------------------------------------------------------------------------------------------------


def _katla(metin: str) -> str:
    """Türkçe harfler ve büyük/küçük ayrımı bir yana; boşluklar tek, ``+`` çevresi boşluksuz."""
    tablo = str.maketrans({"ı": "i", "İ": "i", "I": "i", "ç": "c", "Ç": "c", "ğ": "g", "Ğ": "g", "ö": "o", "Ö": "o",
                           "ş": "s", "Ş": "s", "ü": "u", "Ü": "u"})
    metin = metin.translate(tablo).lower()
    metin = re.sub(r"\s+", " ", metin).strip()
    return re.sub(r"\s*\+\s*", "+", metin)


def _gun_degeri(metin: str | date) -> date | None:
    """``GG.AA.YYYY``, ``G.A.YYYY``, ``GG/AA/YYYY`` ya da ``YYYY-AA-GG``; olmayan bir gün okunmaz."""
    if isinstance(metin, date):
        return metin
    t = str(metin).strip()
    m = re.fullmatch(r"(\d{1,2})[./](\d{1,2})[./](\d{4})", t)
    if m:
        g, a, y = int(m.group(1)), int(m.group(2)), int(m.group(3))
    else:
        m = re.fullmatch(r"(\d{4})-(\d{1,2})-(\d{1,2})", t)
        if not m:
            return None
        y, a, g = int(m.group(1)), int(m.group(2)), int(m.group(3))
    try:
        return date(y, a, g)
    except ValueError:
        return None


def _nufus(metin: str) -> int | None:
    """Bir nüfus: rakamlar, binlikler noktayla ya da boşlukla ayrılabilir (``2.345.678``)."""
    t = re.sub(r"[\s'’_]", "", metin)
    if re.fullmatch(r"\d{1,3}(\.\d{3})+", t):
        t = t.replace(".", "")
    return int(t) if re.fullmatch(r"\d+", t) else None


def _alan(metin: str) -> float | None:
    """Bir alan (m²): ondalık nokta ya da virgülle (``5000.5``, ``5000,5``), binlikler noktayla ve ondalık virgülle
    (``12.500,75``). ``5.000`` gibi tek noktalı ve üç basamaklı yazım belirsizdir (5 mi 5000 mi): okunmaz."""
    t = re.sub(r"[\s'’_]", "", metin)
    if re.fullmatch(r"\d+", t):
        return float(t)
    if re.fullmatch(r"\d{1,3}\.\d{3}", t):
        return None
    if re.fullmatch(r"\d{1,3}(\.\d{3})+(,\d+)?", t):
        return float(t.replace(".", "").replace(",", "."))
    if re.fullmatch(r"\d+[.,]\d+", t):
        return float(t.replace(",", "."))
    return None


def _secim(metin: str, secenekler: list[str]) -> str | None:
    """Seçeneklerden biri: kendisi, tek bir seçeneğin başı (``t`` → tadilat) ya da tek bir seçenekle başlayıp harf ya
    da rakam olmayan bir işaretle süren yazım (``26. madde`` → 26)."""
    t = _katla(metin)
    katli = {_katla(s): s for s in secenekler}
    if t in katli:
        return katli[t]
    bas = [s for k, s in katli.items() if t and k.startswith(t)]
    if len(bas) == 1:
        return bas[0]
    ile = [s for k, s in katli.items() if t.startswith(k) and not t[len(k):len(k) + 1].isalnum()]
    return ile[0] if len(ile) == 1 else None


def _girdiler(permit: dict[str, Any], girdiler: dict[str, dict]) -> tuple[dict[str, Any], list[dict], dict]:
    """Künyenin değerleri okunmuş: bilinenler (tarih ``date``, sayı, seçim sözcüğü; boş tarih ``None``), okunamayanların
    sorunları ve çizimden alınanların kaynağı."""
    yazilan = permit.get("particulars") or {}
    bilinen: dict[str, Any] = {"basvuru_tarihi": None, "ihale_tarihi": None, "epys_karar_tarihi": None}
    sorunlar: list[dict] = []
    for ad, g in girdiler.items():
        metin = str(yazilan.get(ad) or "").strip()
        if not metin:
            continue
        tur = g["kind"]
        if tur == "text":
            bilinen[ad] = metin
            continue
        if tur == "date":
            deger = _gun_degeri(metin)
            cozum = "GG.AA.YYYY yazın (01.09.2027)."
        elif tur == "choice":
            deger = _secim(metin, g["choices"])
            cozum = " ya da ".join(g["choices"]) + " yazın."
        elif ad == "kat_adedi":
            deger = int(metin) if re.fullmatch(r"\d+", metin) else None
            cozum = "Bodrum dışındaki katların sayısını rakamla yazın (3)."
        elif ad == "yapi_insaat_alani":
            deger = _alan(metin)
            cozum = "Alanı m² olarak, binlik ayırmadan yazın (5000 ya da 5000.5)."
        else:
            deger = _nufus(metin)
            cozum = "Nüfusu rakamla yazın (2345678 ya da 2.345.678)."
        if deger is None:
            if tur == "date":
                bilinen.pop(ad, None)
            sorunlar.append({"input": ad, "message": f"{g['label']}: “{metin}” okunamadı. {cozum}"})
        else:
            bilinen[ad] = deger
    kaynak: dict[str, str] = {}
    if "yapi_insaat_alani" not in bilinen and not any(s["input"] == "yapi_insaat_alani" for s in sorunlar):
        olcu = permit.get("constructionArea")
        if isinstance(olcu, (int, float)):
            bilinen["yapi_insaat_alani"] = float(olcu)
            kaynak["yapi_insaat_alani_kaynagi"] = "çizim"
    if "kat_adedi" not in bilinen and not any(s["input"] == "kat_adedi" for s in sorunlar):
        katlar = permit.get("storeys")
        if isinstance(katlar, dict) and isinstance(katlar.get("above"), int):
            bilinen["kat_adedi"] = katlar["above"]
            kaynak["kat_adedi_kaynagi"] = "çizim"
    return bilinen, sorunlar, kaynak


def _goster(deger: Any) -> Any:
    return deger.isoformat() if isinstance(deger, date) else deger


def _adaylar(cal: dict[str, Any], girdiler: dict[str, dict]) -> dict[str, list[Any]]:
    """Eksik bir girdinin denenecek değerleri: her eşiğin kendisi ve bir fazlası (kesin büyük, K23), her seçenek;
    okunamayan bir başvuru ya da ihale tarihi için takvimin her gününün kendisi ve bir önceki günü; e-PYS kararı için
    yürürlük günü ve çok uzak bir gün."""
    il, belediye, alan = set(), set(), {float(cal["scope"]["detached"]["mostArea"])}
    for yukum in cal["obligations"]:
        for evre in yukum.get("stages", []):
            if "province" in evre:
                il.add(evre["province"])
            if "municipality" in evre:
                belediye.add(evre["municipality"])
            if "area" in evre:
                alan.add(float(evre["area"]))
    gunler = {_gun_degeri(y["inForce"]) for y in cal["acts"]} | {_gun_degeri(cal["uses"]["from"])}
    gunler |= {_gun_degeri(e["from"]) for y in cal["obligations"] for e in y.get("stages", [])}
    tarihler = sorted({g for gun in gunler for g in (gun - timedelta(days=1), gun)})
    adaylar: dict[str, list[Any]] = {
        "basvuru_tarihi": tarihler,
        "ihale_tarihi": tarihler,
        "il_nufusu": sorted({v for e in il for v in (e, e + 1)}),
        "belediye_nufusu": sorted({v for e in belediye for v in (e, e + 1)}),
        "yapi_insaat_alani": sorted({v for e in alan for v in (e, e + 1)}),
        "kat_adedi": [0, cal["scope"]["detached"]["mostStoreys"], cal["scope"]["detached"]["mostStoreys"] + 1],
        "epys_karar_tarihi": [_gun_degeri(_yasa(cal, "epys")["inForce"]), _UZAK],
    }
    for ad, g in girdiler.items():
        if g["kind"] == "choice":
            adaylar[ad] = list(g["choices"])
    return adaylar


# --- Keşif: eksik girdinin her değeri ---------------------------------------------------------------------------


class _Gerek(Exception):
    """Karar bilinmeyen bir girdiyi okudu."""

    def __init__(self, ad: str) -> None:
        super().__init__(ad)
        self.ad = ad


class _Degerler:
    """Kararın okuduğu değerler; bilinmeyeni okumak keşfi o girdide dallandırır."""

    def __init__(self, bilinen: dict[str, Any]) -> None:
        self._bilinen = bilinen

    def __getattr__(self, ad: str) -> Any:
        if ad.startswith("_"):
            raise AttributeError(ad)
        if ad in self._bilinen:
            return self._bilinen[ad]
        raise _Gerek(ad)


def _kesfet(yukum: dict, karar_ver: Callable[[_Degerler], tuple], bilinen: dict[str, Any],
            adaylar: dict[str, list[Any]], girdiler: dict[str, dict]) -> dict[str, Any]:
    """Kararı bilinenlerle verir; bilinmeyen bir girdiyi okursa her adayıyla yeniden. Bütün yollar aynı durumu, başlangıcı
    ve bitişi veriyorsa sonuç odur; vermiyorsa ``confirm``."""
    yigin = [dict(bilinen)]
    sonuclar: dict[tuple, tuple] = {}
    sonuclar_gunleri: dict[str, set] = {}
    yollar: list[tuple] = []
    eksik: list[str] = []
    yol = 0
    while yigin:
        yol += 1
        if yol > EN_COK_YOL:
            sonuclar[("confirm", None, None)] = ("confirm", None, None, (), "Girdilerin çok azı verilmiş.")
            break
        degerler = yigin.pop()
        try:
            sonuc = karar_ver(_Degerler(degerler))
        except _Gerek as g:
            if g.ad not in eksik:
                eksik.append(g.ad)
            for aday in reversed(adaylar.get(g.ad, [None])):
                yigin.append(degerler | {g.ad: aday})
            continue
        sonuclar.setdefault(_anahtar(sonuc), sonuc)
        sonuclar_gunleri.setdefault(sonuc[0], set()).add(sonuc[1:3])
        if len(yollar) < 64:
            yollar.append(sonuc)
        if len(sonuclar) > 1:
            break
    sira = list(girdiler)
    eksik.sort(key=lambda ad: sira.index(ad) if ad in sira else len(sira))
    adlar = ", ".join(girdiler[a]["label"].split(" (")[0] for a in eksik if a in girdiler)
    if len(sonuclar) == 1:
        durum, bas, bit, maddeler, neden = next(iter(sonuclar.values()))
        gunler = sonuclar_gunleri.get(durum, set())
        if len(gunler) > 1:
            # Uygulanır ya da uygulanmaz her yolda; başladığı gün eksik girdiye bağlı: o gün yazılmaz.
            bas, bit = None, None
            maddeler = tuple(dict.fromkeys(m for g in yollar for m in g[3]))
            neden = (f"{DURUMLAR[durum]}; hükmün bu yapıya başladığı gün girilmeyen bilgilere bağlı, sonucu "
                     f"değiştirmez: {adlar}.")
        elif eksik:
            neden += f" Girilmeyenler sonucu değiştirmez: {adlar}."
    else:
        durum, bas, bit = "confirm", None, None
        maddeler = tuple(dict.fromkeys(m for s in sonuclar.values() for m in s[3]))
        neden = f"Sonuç girilmeyen bilgilere bağlı: {adlar}. Künyeye yazın." if adlar else "Girdiler yetmiyor."
    maddeler = tuple(dict.fromkeys(maddeler))
    return {
        "id": yukum["id"],
        "title": yukum["title"],
        "state": durum,
        "words": DURUMLAR[durum],
        "from": bas.isoformat() if bas else None,
        "until": bit.isoformat() if bit else None,
        "articles": list(maddeler),
        "reason": neden,
        "missing": eksik if durum == "confirm" else [],
    }


def _anahtar(sonuc: tuple) -> tuple:
    """İki yolun aynı sonuç sayıldığı şey: uygulanır ve uygulanmaz için yalnız durum (başladığı gün eksik girdiye bağlı
    olabilir, sonuç değişmez); henüz başlamadı için durum ve başlangıç günü (ne zaman sorusunun cevabı odur); teyit için
    durum."""
    return (sonuc[0], sonuc[1]) if sonuc[0] == "not_yet" else (sonuc[0],)


# --- Kararlar ---------------------------------------------------------------------------------------------------


def _atif(kaynak: dict[str, Any]) -> str:
    """Kaynağın atfı, kataloğun yazdığı gibi: “Md. 4/5”, “Geçici Md. 1/1-a”, “e-PYS Md. 4/4”."""
    parcalar = [kaynak["act"]] if kaynak.get("act") else []
    madde = kaynak.get("article", "")
    parcalar.append(f"Geçici Md. {madde[len('Geçici '):]}" if madde.startswith("Geçici ") else f"Md. {madde}")
    return " ".join(parcalar)


def _tarih(gun: date) -> str:
    return f"{gun.day}.{gun.month}.{gun.year}"


def _yasa(cal: dict, kimlik: str) -> dict:
    return next(y for y in cal["acts"] if y["id"] == kimlik)


def _yukum(cal: dict, kimlik: str) -> dict:
    return next(y for y in cal["obligations"] if y["id"] == kimlik)


def _konut(d: _Degerler, cal: dict) -> bool:
    return d.kullanim in cal["uses"]["residential"]


def _kapsam_disi(d: _Degerler, cal: dict) -> tuple | None:
    """Md. 1/2: kapsamın dışında kalan yapı; ``None`` kapsam içidir. Okuma sırası keşfi kısa tutar: önce plan, yapı
    türü, sonra müstakil yapının katı ve alanı, en son kullanım ve tarım."""
    s = cal["scope"]
    if d.plan_alani == "hayır":
        return ("not_applicable", None, None, (_atif(s["plan"]["source"]),),
                "Uygulama imar planı bulunmayan alanda: yönetmelik plan alanındaki yeni binaları kapsar.")
    if d.yapi_turu == "tadilat":
        return ("not_applicable", None, None, (_atif(s["newBuilding"]["source"]),),
                "Yönetmelik yeni yapılacak binaları kapsar; tadilat projesi kapsamda değil (K28).")
    if d.yapi_turu == "ilave":
        return ("confirm", None, None, (_atif(s["newBuilding"]["source"]),),
                "Yönetmelik yeni yapılacak binaları kapsar; ilavenin kapsamda olup olmadığını idare söyler (K28).")
    mustakil = s["detached"]
    if d.mustakil == "evet" and d.kat_adedi <= mustakil["mostStoreys"] and d.yapi_insaat_alani <= mustakil["mostArea"]:
        return ("not_applicable", None, None, (_atif(mustakil["source"]),),
                f"Bodrum dışında en çok {mustakil['mostStoreys']} katlı ve toplam yapı inşaat alanı "
                f"{mustakil['mostArea']:g} m²'yi geçmeyen müstakil yapı.")
    if not _konut(d, cal) and d.tarim_hayvancilik == "evet":
        return ("not_applicable", None, None, (_atif(s["farm"]["source"]),),
                "Entegre tesis niteliğinde olmayan tarım ve hayvancılık yapısı.")
    return None


def _evre(d: _Degerler, yukum: dict) -> tuple[date, dict]:
    """Yapının girdiği ilk evre: eşiklerinin hepsini kesin geçtiği (K23). Önce alan okunur: alan eşiğin altındaysa
    nüfus gerekmez."""
    for evre in yukum["stages"]:
        if "area" in evre and not d.yapi_insaat_alani > evre["area"]:
            continue
        if "province" in evre and not d.il_nufusu > evre["province"]:
            continue
        if "municipality" in evre and not d.belediye_nufusu > evre["municipality"]:
            continue
        return _gun_degeri(evre["from"]), evre
    raise ValueError(f"{yukum['id']}: son evrenin eşiği olmamalı")


def _yurur(cal: dict) -> date:
    return _gun_degeri(_yasa(cal, "hazirlama")["inForce"])


def _baslar(d: _Degerler, cal: dict) -> tuple[date, list[str]]:
    """Yönetmeliğin bu yapıya başladığı gün: yürürlük; konut ve ticaret+konut dışında Geçici 1/2'nin günü."""
    bas, maddeler = _yurur(cal), [_atif(_yasa(cal, "hazirlama")["source"])]
    if not _konut(d, cal):
        bas = max(bas, _gun_degeri(cal["uses"]["from"]))
        maddeler.append(_atif(cal["uses"]["source"]))
    return bas, maddeler


def _basvuru(d: _Degerler) -> date | None:
    """Esas alınan gün: başvuru ile ihale tarihinden erken olanı (Geçici 2 ikisini de sayar; K25). Hiçbiri yoksa
    başvuru henüz yapılmamıştır."""
    gunler = [g for g in (d.basvuru_tarihi, d.ihale_tarihi) if g is not None]
    return min(gunler) if gunler else None


def _once(d: _Degerler, cal: dict) -> tuple | None:
    """Başvuru yönetmeliğin yürürlüğünden önceyse yönetmelik ona uygulanmaz (Md. 1/1 "yapılacak müracaatlar", Md. 16;
    K25)."""
    bas = _basvuru(d)
    yurur = _yurur(cal)
    if bas is not None and bas < yurur:
        return ("not_applicable", None, None,
                (_atif(cal["scope"]["applications"]["source"]), _atif(_yasa(cal, "hazirlama")["source"])),
                f"Başvuru ({_tarih(bas)}) yönetmeliğin yürürlüğünden ({_tarih(yurur)}) önce yapılmış.")
    return None


def _baslangic(d: _Degerler, cal: dict, kimlik: str) -> tuple[date, list[str]]:
    """Yükümlülüğün bu yapıya başladığı gün ve dayandığı maddeler: evresinin günü ile yönetmeliğin bu yapıya başladığı
    günden geç olanı."""
    gun, evre = _evre(d, _yukum(cal, kimlik))
    bas, maddeler = _baslar(d, cal)
    return max(gun, bas), [_atif(evre["source"]), *maddeler[1:]]


def _kapsam(d: _Degerler, cal: dict, bugun: date) -> tuple:
    disi = _kapsam_disi(d, cal) or _once(d, cal)
    if disi:
        return disi
    bas, maddeler = _baslar(d, cal)
    return _tarihli(d, cal, bugun, bas, None, (_atif(cal["scope"]["plan"]["source"]), *maddeler),
                    "Uygulama imar planı alanında yeni bina.",
                    ozne=("Yönetmelik", "yönetmeliğin"))


def _onceki(cal: dict, bas: date) -> str:
    """Başlangıcından önceki başvuruya hükmün uygulanmamasının dayanağı: yürürlük günü için Md. 1/1 ("yapılacak
    müracaatlar", K25), geçici maddenin günleri için Geçici Md. 2."""
    if bas == _yurur(cal):
        return _atif(cal["scope"]["applications"]["source"])
    return _atif(cal["earlier"]["source"])


def _tarihli(d: _Degerler, cal: dict, bugun: date, bas: date, bit: date | None, maddeler: tuple, ne: str,
             sonra: str = "", ozne: tuple[str, str] = ("Hüküm", "hükmün")) -> tuple:
    """Başlangıcı ``bas`` olan bir hükmün bu yapıya durumu. Başvuru tarihi varsa ona göre (önceki başvuruya
    uygulanmaz); yoksa başvuru bugün ya da sonra yapılacaktır. ``sonra`` sonuna eklenen not; ``ozne`` hükmün adı ve
    tamlayan hâli."""
    adi, adinin = ozne
    bas_v = _basvuru(d)
    if bas_v is not None:
        if bas_v < bas:
            return ("not_applicable", None, None, (*maddeler, _onceki(cal, bas)),
                    f"Başvuru ({_tarih(bas_v)}) {adinin} bu yapıya başladığı {_tarih(bas)} tarihinden önce yapılmış.")
        return ("applies", bas, None, maddeler,
                f"{ne} Başvuru {_tarih(bas_v)} tarihli; {adi.lower()} bu yapıya {_tarih(bas)} tarihinde "
                f"başladı.{sonra}")
    if bas <= bugun:
        return ("applies", bas, bit, maddeler, f"{ne} {adi} bu yapıya {_tarih(bas)} tarihinde başladı.{sonra}")
    return ("not_yet", bas, bit, maddeler,
            f"{ne} {adi} bu yapıya {_tarih(bas)} tarihinde başlar; o günden önce yapılan başvuruya uygulanmaz "
            f"({_onceki(cal, bas)}).{sonra}")


def _pdfa(d: _Degerler, cal: dict, bugun: date) -> tuple:
    disi = _kapsam_disi(d, cal) or _once(d, cal)
    if disi:
        return disi
    yukum = _yukum(cal, "pdfa")
    p_bas, p_madde = _baslangic(d, cal, "pdfa")
    i_bas, i_madde = _baslangic(d, cal, "ifc")
    biter = _atif(yukum["endsWith"]["source"])
    maddeler = (*(_atif(k) for k in yukum["sources"]), *p_madde)
    devir = (*maddeler, biter, *i_madde)
    model = "modelden üretilen çizimler Md. 9/1 ile PDF/A teslim edilir."
    bas_v = _basvuru(d)
    if bas_v is not None:
        if bas_v >= i_bas:
            return ("not_applicable", None, None, devir,
                    f"Başvuru ({_tarih(bas_v)}) IFC hükmünün bu yapıya başladığı {_tarih(i_bas)} tarihinden sonra: "
                    f"4/2–3 uygulanmaz ({biter} son cümle); {model}")
        return _tarihli(d, cal, bugun, p_bas, None, maddeler, "2B çizim dosyaları ve belgeler PDF/A.")
    if i_bas <= bugun:
        return ("not_applicable", None, None, devir,
                f"IFC hükmü bu yapıya {_tarih(i_bas)} tarihinde başladı; 4/2–3 artık uygulanmaz ({biter} son cümle); "
                f"{model}")
    if p_bas >= i_bas:
        return ("not_applicable", None, None, devir,
                f"4/2–3 bu yapıya IFC hükmüyle aynı gün ({_tarih(i_bas)}) başlayacağından kendi başına hiç uygulanmaz "
                f"({biter} son cümle; K26); {model}")
    sonra = f" {_tarih(i_bas)} ve sonrasındaki başvuruda yerini IFC ile Md. 9/1 alır ({biter} son cümle)."
    return _tarihli(d, cal, bugun, p_bas, i_bas, maddeler, "2B çizim dosyaları ve belgeler PDF/A.", sonra)


def _ifc(d: _Degerler, cal: dict, bugun: date) -> tuple:
    disi = _kapsam_disi(d, cal) or _once(d, cal)
    if disi:
        return disi
    yukum = _yukum(cal, "ifc")
    bas, madde = _baslangic(d, cal, "ifc")
    return _tarihli(d, cal, bugun, bas, None, (*(_atif(k) for k in yukum["sources"]), *madde),
                    "BIM modeli IFC biçiminde teslim edilir.")


def _modelden(d: _Degerler, cal: dict, bugun: date) -> tuple:
    """Md. 9/1, IFC teslimiyle başlar (K26): durumu ve günü IFC'ninkidir, maddesi kendisinin."""
    sonuc = _ifc(d, cal, bugun)
    yukum = _yukum(cal, "modelden-pdfa")
    if sonuc[0] not in ("applies", "not_yet"):
        return (*sonuc[:3], (*(_atif(k) for k in yukum["sources"]), *sonuc[3]), sonuc[4])
    maddeler = (*(_atif(k) for k in yukum["sources"]), *sonuc[3][1:])
    ne = "BIM modelinden üretilen CAD çizimleri PDF/A teslim edilir; IFC teslimiyle başlar."
    return _tarihli(d, cal, bugun, sonuc[1], None, maddeler, ne)


def _epys(d: _Degerler, cal: dict, bugun: date) -> tuple:
    """e-PYS: aynı kapsam ve İmar Kanunu'nun 4, 26, 27. maddelerindeki yapılar dışında (e-PYS Md. 1/2); yürürlük
    1.9.2028, fiilî başlangıç Bakanlığın kararıyla (Md. 4/4). Hazırlama yönetmeliğinin geçici maddeleri e-PYS'yi
    bağlamaz; karardan önce yapılan başvuru e-PYS'den geçmez."""
    disi = _kapsam_disi(d, cal)
    if disi:
        return disi
    yururluk = _gun_degeri(_yasa(cal, "epys")["inForce"])
    yukum = _yukum(cal, "epys")
    maddeler = (*(_atif(k) for k in yukum["sources"]), _atif(_yasa(cal, "epys")["source"]),
                _atif(yukum["decision"]["source"]))
    bas_v = _basvuru(d)
    if bas_v is not None and bas_v < yururluk:
        return ("not_applicable", None, None, maddeler,
                f"Başvuru ({_tarih(bas_v)}) e-PYS Yönetmeliğinin yürürlüğünden ({_tarih(yururluk)}) önce yapılmış.")
    yasa = cal["scope"]["lawArticles"]
    if d.kanun_istisnasi in yasa["articles"]:
        return ("not_applicable", None, None, (_atif(yasa["source"]),),
                f"İmar Kanunu'nun {d.kanun_istisnasi}. maddesindeki yapı: e-PYS'nin kapsamı dışında.")
    karar_gunu = d.epys_karar_tarihi
    if karar_gunu is None:
        # Fiilî başlangıç Bakanlığın kararıyladır (Md. 4/4): girilmemiş karar, olabilecek her gün denenerek okunur.
        raise _Gerek("epys_karar_tarihi")
    bas = max(yururluk, karar_gunu)
    if bas_v is not None and bas_v < bas:
        return ("not_applicable", None, None, maddeler,
                f"Başvuru ({_tarih(bas_v)}) e-PYS'nin bu yapıya başladığı {_tarih(bas)} tarihinden önce yapılmış.")
    if bas_v is not None or bas <= bugun:
        return ("applies", bas, None, maddeler, f"Proje e-PYS'de teslim edilir; {_tarih(bas)} tarihinden beri.")
    return ("not_yet", bas, None, maddeler, f"e-PYS bu yapıya {_tarih(bas)} tarihinde başlar.")


_KARARLAR: dict[str, Callable[[_Degerler, dict, date], tuple]] = {
    "kapsam": _kapsam,
    "pdfa": _pdfa,
    "ifc": _ifc,
    "modelden-pdfa": _modelden,
    "epys": _epys,
}
