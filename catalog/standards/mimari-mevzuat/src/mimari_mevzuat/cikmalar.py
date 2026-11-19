"""Çıkmalar: Planlı Alanlar İmar Yönetmeliği 41 (docs/adr/8152).

KentOS zemin katın üstündeki katlarda mahallerin taban alanı (zemin kat izi) dışındaki parçalarını çıkma olarak verir
(``building.projections``): açık çıkma dış mekândır (balkon, teras), kapalı çıkma iç mekân; her biri vaziyet planının
parsel kenarlarına en yakın uzaklığıyla, kenarın istediği bahçe mesafesiyle ve altındaki döşemenin alt kotuyla.
Vaziyet planı yoksa çıkma okunmaz.

- **41/1 (1.10.2017):** çıkmalar kendi bahçe hudutları dışına taşmaz. Yol cephesinde (ön bahçe) parsel sınırları içinde
  yapı yaklaşma sınırından en çok 1,50 m taşar (a-1, b-1); yan ve arka bahçede parsel sınırına 3,00 m'den fazla
  yaklaşmamak kaydıyla 1,50 m taşar (a-2, b-2); taban yapı yaklaşma sınırından içeri çekilmişse ölçü yine yapı yaklaşma
  sınırından alınır (a-3, b-3): KentOS taşmayı kenarın istediği mesafeden ölçer, zemin kattan değil. Bitişik nizamda
  açık çıkma bitişik olduğu komşu sınırına 2,00 m'den fazla yaklaşamaz (b-4). Kamusal alana komşu kenar (park, meydan)
  yol cephesi değildir: yan ve arka bahçe gibi okunur (sıkı taraf).
- **41/2:** açık ve kapalı çıkmaların tabii ya da tesviye edilmiş zeminden çıkma altına kadar düşey mesafesi en az 2,40
  m. Zemin modelin ±0,00'ıdır; ön bahçesi 3 m ve daha az parselde yol kotu da ±0,00 sayılır. Altında döşeme çizilmemiş
  çıkmanın katının kotu 2,40 m'den azsa kaldı, değilse eksik veri.

**Okunan olgular** (``facts.building``): ``projections`` (mahal, kat, açık mı, alanı, katın kotu, alt kotu, kenarlar:
türü, istenen mesafe, uzaklık), katların adları.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import katman
from . import yururluk as y

MADDELER = [("PAİY", "41/1"), ("PAİY", "41/2")]

_YAN = ("side", "back", "public")


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _m(x: float) -> str:
    """Bir uzunluk iletide: iki ondalık (çizimin sayıları gibi)."""
    return katman.metre(x)


def _ne(c: dict[str, Any], katlar: dict[str, str]) -> str:
    """Çıkmanın adı iletide: katı ve türü."""
    kat = katlar.get(c.get("level") or "", "Katsız")
    return f"{kat}: {'açık' if c.get('open') else 'kapalı'} çıkma ({c['area']:.2f} m²)"


def tasma(c: dict[str, Any], hal: y.Hal) -> list[str]:
    """41/1'e göre çıkmanın aşan kenarları, her biri sözüyle; tutuyorsa boş."""
    en_cok, uzak, komsu = float(hal.anahtar["over"]), float(hal.anahtar["clear"]), float(hal.anahtar["party"])
    out = []
    for e in c.get("edges") or []:
        tur, istenen, uzaklik = e["kind"], float(e["required"]), float(e["distance"])
        icinde = uzaklik < istenen - 1e-9
        tasan = istenen - uzaklik
        if tur == "front" and icinde and tasan > en_cok + 1e-9:
            out.append(f"{e['edge']}. kenar (ön bahçe): yapı yaklaşma sınırından {_m(tasan)} taşıyor, en çok "
                       f"{_m(en_cok)}")
        elif tur in _YAN and icinde and (tasan > en_cok + 1e-9 or uzaklik < uzak - 1e-9):
            neden = []
            if tasan > en_cok + 1e-9:
                neden.append(f"bahçe mesafesinden {_m(tasan)} taşıyor, en çok {_m(en_cok)}")
            if uzaklik < uzak - 1e-9:
                neden.append(f"parsel sınırına {_m(uzaklik)}, en az {_m(uzak)}")
            out.append(f"{e['edge']}. kenar ({'arka' if tur == 'back' else 'yan' if tur == 'side' else 'kamusal alan'} "
                       f"bahçe): " + ", ".join(neden))
        elif tur == "party" and c.get("open") and uzaklik < komsu - 1e-9:
            out.append(f"{e['edge']}. kenar (bitişik komşu): açık çıkma komşu sınırına {_m(uzaklik)}, en az "
                       f"{_m(komsu)}")
    return out


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin çıkma bulguları ve 41/1, 41/2'nin satırları."""
    cikmalar = bina.get("projections") or []
    h1, h2 = y.hal("PAİY", "41/1", gun), y.hal("PAİY", "41/2", gun)
    if h1 is None or h2 is None or not cikmalar:
        if h1 is None or h2 is None:
            return y.Sonuc([], [])
        not_ = "zemin katın dışına taşan çıkma yok (ya da vaziyet planı yok)"
        return y.Sonuc([], [_satir(h1, y.KAPSAM_DISI, not_), _satir(h2, y.KAPSAM_DISI, not_)])
    katlar = {k["level"]: k.get("name") or k["level"] for k in bina.get("storeys") or []}
    bulgular: list[dict[str, Any]] = []
    kalan1 = 0
    for c in cikmalar:
        asanlar = tasma(c, h1)
        if asanlar:
            kalan1 += 1
            bulgular.append({
                "code": "paiy_projection_over",
                "message": f"{_ne(c, katlar)}: " + "; ".join(asanlar) + f" — {h1.kisa()}",
                "repair": "Çıkmayı kısaltın ya da yapı yaklaşma sınırına çekin: yol cephesinde en çok 1,50 m, yan ve arka "
                          "bahçede en çok 1,50 m ve parsel sınırına en az 3,00 m.",
                "uids": [c["room"]], "rule": h1.kural(), "_iz": f"41/1:{c['room']}"})
    satir1 = (_satir(h1, y.KALDI, f"{kalan1} çıkma bahçe mesafesinden fazla taşıyor") if kalan1
              else _satir(h1, y.GECTI, f"{len(cikmalar)} çıkma, hepsi sınırlar içinde"))
    en_az = float(h2.anahtar["height"])
    kalan2, eksik2 = 0, 0
    for c in cikmalar:
        alt = c.get("underside")
        if alt is not None and float(alt) < en_az - 1e-9:
            kalan2 += 1
            bulgular.append({
                "code": "paiy_projection_low",
                "message": f"{_ne(c, katlar)}: çıkma altı zeminden {_m(float(alt))}, en az {_m(en_az)} — {h2.kisa()}",
                "repair": "Çıkmayı yükseltin: zeminden çıkma altına en az 2,40 m.", "uids": [c["room"]],
                "rule": h2.kural(), "_iz": f"41/2:{c['room']}"})
        elif alt is None and float(c["floor"]) < en_az - 1e-9:
            kalan2 += 1
            bulgular.append({
                "code": "paiy_projection_low",
                "message": f"{_ne(c, katlar)}: katın kotu {_m(float(c['floor']))}, çıkma altı ondan da alçak; en az "
                           f"{_m(en_az)} — {h2.kisa()}",
                "repair": "Çıkmayı yükseltin: zeminden çıkma altına en az 2,40 m.", "uids": [c["room"]],
                "rule": h2.kural(), "_iz": f"41/2:{c['room']}"})
        elif alt is None:
            eksik2 += 1
            bulgular.append({
                "code": "paiy_projection_low",
                "message": f"{_ne(c, katlar)}: altında döşeme çizilmemiş, çıkma altının zeminden yüksekliği okunmuyor "
                           f"— {h2.kisa()}",
                "repair": "Çıkmanın döşemesini çizin (Döşeme): alt kotu ondan okunur.", "uids": [c["room"]],
                "rule": h2.kural(), "_iz": f"41/2:{c['room']}:yok", "state": y.EKSIK_VERI})
    if kalan2:
        satir2 = _satir(h2, y.KALDI, f"{kalan2} çıkmanın altı 2,40 m'den alçak")
    elif eksik2:
        satir2 = _satir(h2, y.EKSIK_VERI, f"{eksik2} çıkmanın altında döşeme yok")
    else:
        satir2 = _satir(h2, y.GECTI, f"{len(cikmalar)} çıkmanın altı en az {_m(en_az)}")
    return y.Sonuc(bulgular, [satir1, satir2])


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin çıkma bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "çıkma kuralı")
