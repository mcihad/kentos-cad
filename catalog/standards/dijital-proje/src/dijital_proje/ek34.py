"""EK-3 ve EK-4 denetimi (docs/adr/8064; plan/bim-plan.md BIM-03; Dijital Hazırlama Yönetmeliği Md. 7): paftaya girecek
2B çizimde mahal numarası ve adı (EK-3), katman, tarama, aks çizgisi, ölçü yazısı ve blok adı (EK-4).

Kural çizimi okumaz: KentOS'un ölçtüğü olgulara (``facts.project.cad``) bakar. Kataloğun satırları ``data/ek34-2026.json``'dadır
(``scripts/mevzuat/ek34_veri.py`` KentOS'un kural kataloğundan yazar; elle düzenlenmez). Her bulgu kuralını, okunduğu
maddeyi ve sayfasını, düzelten tek adımı (varsa) söyler.

Kurallar:

- ``number_pattern`` — mahal numarası ``Kat_Bölüm_Sıra``: üçer karakter, alt tireyle (3.1.1, Tablo 3.1).
- ``storey_code`` — numaranın kat alanı mahalin katının kodu (3.2.1; EK-2 Tablo 2.14, paftaların adıyla aynı kod).
- ``sequence`` — her kat ve bölüm için sıra 001'den başlar, boşluksuz ve tekrarsız artar (3.2.3).
- ``clockwise`` — sıra saat yönünde artar (3.2.3).
- ``room_name`` — Tablo 3.2'deki bir ad, yazımı tablodaki gibi (3.3). Tabloda olmayan ad projeye özgü sayılır, bulgu
  değildir.
- ``layer_name`` — nesne taşıyan katmanın adı büyük harf, Türkçe harfsiz, alanlar orta tireyle; disiplinin şablonuna göre
  disiplin, öge ya da alt disiplin, malzeme (4.11; Tablo 4.1–4.3; 4.10: en çok 15 karakter).
- ``layer_zero`` — "0" katmanında çizim yapılmaz (4.12.1).
- ``hatch_layer`` — tarama, adının sonu "-T" olan ayrı bir katmanda (4.14.5).
- ``emsal_layer`` — EMSL katmanının malzemesi DAHIL, HARIC ya da DIGER (4.14.2).
- ``axis_line`` — aks çizgileri noktalı kesikli (4.29).
- ``dimension_font`` — ölçü yazısı Times New Roman, Arial, Calibri, Romans ya da Tahoma (4.21.2). Arimo, Arial'ın
  ölçüleriyle aynı açık yazı tipidir: KentOS onu idarenin teyidine bırakır (K29).
- ``block_name`` — blok adı Tablo 4.4'ün şablonunda: disiplin, öge, isteğe bağlı malzeme ve ebat (4.18).
"""

from __future__ import annotations

import json
import math
import re
from importlib import resources
from typing import Any

#: The article and the gazette every finding names.
MADDE = "Dijital Proje Yön. 7"
RG = "RG 5.8.2026/33331"

#: The tadilat states' suffixes KentOS writes after an EK-4 layer name (docs/adr/8039 §4; K37).
TADILAT = ("MEVCUT", "IPTAL", "ILAVE", "GECICI")

_TR = str.maketrans({"ı": "i", "İ": "i", "I": "i", "ç": "c", "Ç": "c", "ğ": "g", "Ğ": "g", "ö": "o", "Ö": "o",
                     "ş": "s", "Ş": "s", "ü": "u", "Ü": "u"})


def veri() -> dict[str, Any]:
    """Paketteki ``ek34-2026.json``."""
    return json.loads(resources.files(__package__).joinpath("data/ek34-2026.json").read_text(encoding="utf-8"))


def denetle(facts: dict[str, Any], data: dict[str, Any] | None = None) -> list[dict[str, Any]]:
    """``facts``: ``kentos.auditFacts`` (``cad`` bölümü okunur). Bulgular: ``rule``, ``code``, ``severity``, ``message``,
    ``repair``, ``uids``, ``at``, ``cite`` (madde, Resmî Gazete, ekin maddesi ve sayfası), ``fix`` (KentOS komutu ve
    girdisi; yoksa ``None``)."""
    d = data if data is not None else veri()
    cad = facts.get("cad") or {}
    bulgular: list[dict[str, Any]] = []
    bulgular += _odalar(cad, d)
    bulgular += _katmanlar(cad, d)
    bulgular += _olcu(cad, d)
    bulgular += _bloklar(cad, d)
    return bulgular


# --- Ortak --------------------------------------------------------------------------------------------------------


def _katla(metin: str) -> str:
    return re.sub(r"\s+", " ", metin.translate(_TR).lower()).strip()


def _kucuk(metin: str) -> str:
    """Türkçe büyük/küçük harf ayrımı bir yana (İ→i, I→ı)."""
    return re.sub(r"\s+", " ", metin.replace("İ", "i").replace("I", "ı").lower()).strip()


def _ascii_buyuk(metin: str) -> str:
    """4.11: büyük harf, Ç Ğ İ Ö Ş Ü yerine C G I O S U."""
    return metin.translate(_TR).upper()


def _bulgu(d: dict, kural: str, kod: str, mesaj: str, onar: str, uids: list[str], at, fix=None,
           onem: str = "warning") -> dict[str, Any]:
    kaynak = " · ".join([MADDE, RG, *d["rules"][kural]])
    return {
        "rule": kural,
        "code": kod,
        "severity": onem,
        "message": f"{mesaj} — {MADDE}",
        "repair": onar,
        "uids": list(uids),
        "at": [float(at[0]), float(at[1])] if at else [0.0, 0.0],
        "cite": kaynak,
        "fix": fix,
    }


# --- EK-3: mahaller -----------------------------------------------------------------------------------------------

_NUMARA = re.compile(r"^([A-Z0-9]{3})_([A-Z0-9]{3})_([0-9]{3})$")


def _odalar(cad: dict, d: dict) -> list[dict]:
    out: list[dict] = []
    odalar = [r for r in cad.get("rooms") or [] if (r.get("number") or "").strip()]
    gruplar: dict[tuple[str, str, str], list[dict]] = {}
    for r in odalar:
        numara = r["number"].strip()
        m = _NUMARA.match(numara)
        if not m:
            out.append(_bulgu(
                d, "number_pattern", "digital_ek3_number_pattern",
                f"“{numara}” EK-3 numarası değil: Kat_Bölüm_Sıra, üçer karakter (K00_D01_001)",
                "Mahal numarala'da EK-3'ü seçip katın mahallerini yeniden numaralayın ya da numarayı Öznitelikler'de "
                "düzeltin.",
                [r["uid"]], r.get("at")))
            continue
        kat, bolum, sira = m.groups()
        kod = r.get("levelCode")
        if kod and kat != kod:
            yeni = f"{kod}_{bolum}_{sira}"
            out.append(_bulgu(
                d, "storey_code", "digital_ek3_storey_code",
                f"“{numara}” numarasının kat alanı {kat}, mahalin katının kodu {kod} (paftaların adındaki kod)",
                f"Numarayı {yeni} yapın.",
                [r["uid"]], r.get("at"),
                fix={"label": f"Numarayı {yeni} yap", "command": "cad.spaces.edit",
                     "input": {"uids": [r["uid"]], "operation": {"kind": "set", "number": yeni}}}))
        gruplar.setdefault((r.get("level") or "", kat, bolum), []).append({**r, "_sira": int(sira)})
    for (_, kat, bolum), uyeler in sorted(gruplar.items()):
        out += _sira(d, kat, bolum, uyeler)
    out += _adlar(cad, d)
    return out


def _saat_yonu(uyeler: list[dict]) -> list[dict]:
    """Mahaller merkezlerinin ortası çevresinde, kuzeyden başlayıp saat yönünde."""
    cx = sum(r["at"][0] for r in uyeler) / len(uyeler)
    cy = sum(r["at"][1] for r in uyeler) / len(uyeler)

    def aci(r):
        # Kuzeyden saat yönünde: atan2(doğu, kuzey), 0…2π.
        a = math.atan2(r["at"][0] - cx, r["at"][1] - cy)
        return (a + 2 * math.pi) % (2 * math.pi)

    return sorted(uyeler, key=lambda r: (round(aci(r), 9), r["uid"]))


def _sira(d: dict, kat: str, bolum: str, uyeler: list[dict]) -> list[dict]:
    out: list[dict] = []
    siralar = sorted(r["_sira"] for r in uyeler)
    ilk = f"{kat}_{bolum}_001"
    duzeni = _saat_yonu(uyeler)
    fix = {"label": f"{kat}_{bolum} mahallerini saat yönünde yeniden numarala", "command": "cad.spaces.number",
           "input": {"uids": [r["uid"] for r in duzeni], "first": ilk, "order": {"kind": "input"}}}
    uids = [r["uid"] for r in sorted(uyeler, key=lambda r: (r["_sira"], r["uid"]))]
    at = sorted(uyeler, key=lambda r: (r["_sira"], r["uid"]))[0].get("at")
    if siralar != list(range(1, len(siralar) + 1)):
        tekrar = sorted({s for s in siralar if siralar.count(s) > 1})
        neden = (f"{', '.join(f'{s:03d}' for s in tekrar)} birden çok mahalde" if tekrar
                 else f"sıra {', '.join(f'{s:03d}' for s in siralar)}; 001'den boşluksuz olmalı")
        out.append(_bulgu(
            d, "sequence", "digital_ek3_sequence",
            f"{kat}_{bolum} mahallerinin sıra numarası: {neden}",
            "Katın bu bölümdeki mahallerini 001'den saat yönünde yeniden numaralayın.",
            uids, at, fix=fix))
        return out
    if len(uyeler) >= 3:
        sirayla = [r["uid"] for r in sorted(uyeler, key=lambda r: r["_sira"])]
        saat = [r["uid"] for r in duzeni]
        # Saat yönünde bir döngü: 001 nereden başlarsa başlasın, sıra döngünün sırasıdır.
        k = saat.index(sirayla[0])
        if saat[k:] + saat[:k] != sirayla:
            out.append(_bulgu(
                d, "clockwise", "digital_ek3_clockwise",
                f"{kat}_{bolum} mahallerinin numaraları saat yönünde artmıyor",
                "Katın bu bölümdeki mahallerini kuzeyden saat yönünde yeniden numaralayın.",
                uids, at, fix=fix))
    return out


def _adlar(cad: dict, d: dict) -> list[dict]:
    out: list[dict] = []
    tablo = d["rooms"]
    yazim = {_kucuk(t["name"]): t["name"] for t in tablo}
    katli = {_katla(t["name"]): t["name"] for t in tablo}
    for r in cad.get("rooms") or []:
        ad = (r.get("name") or "").strip()
        if not ad:
            continue
        # "Yatak odası 2": sıra eki adın parçası değildir.
        govde = re.sub(r"\s*\d+$", "", ad).strip()
        if not govde or _kucuk(govde) in yazim:
            continue
        dogru = katli.get(_katla(govde))
        if dogru is None:
            continue
        yeni = ad.replace(govde, dogru, 1)
        out.append(_bulgu(
            d, "room_name", "digital_ek3_room_name",
            f"Mahal adı “{ad}” Tablo 3.2'deki yazımla değil: “{dogru}”",
            f"Adı “{yeni}” yapın.",
            [r["uid"]], r.get("at"),
            fix={"label": f"Adı “{yeni}” yap", "command": "cad.spaces.edit",
                 "input": {"uids": [r["uid"]], "operation": {"kind": "set", "name": yeni}}}))
    return out


# --- EK-4: katmanlar ----------------------------------------------------------------------------------------------

_ALAN = re.compile(r"^[A-Z0-9]+$")

#: The facts' line types as the findings say them.
_CIZGI = {"continuous": "sürekli", "dashed": "kesikli", "dashdot": "noktalı kesikli", "dotted": "noktalı"}


def _katman_adi(ad: str, d: dict) -> str | None:
    """Katman adının EK-4'e uymayan yanı; uyuyorsa ``None``."""
    if ad != ad.strip() or " " in ad:
        return "boşluk var"
    if ad != _ascii_buyuk(ad):
        return "küçük harf ya da Türkçe harf var (büyük harf, Ç Ğ İ Ö Ş Ü yerine C G I O S U)"
    alanlar = ad.split("-")
    if any(not a for a in alanlar):
        return "alanlar arasında tek orta tire olmalı"
    if any(not _ALAN.match(a) for a in alanlar):
        return "alanlarda harf ve rakam dışında işaret var"
    # Sonek: tarama "-T" (4.14.5), derinlik 1–4 (4.14.3), tadilat durumu (K37).
    while len(alanlar) > 3 and (alanlar[-1] in ("T", "1", "2", "3", "4") or alanlar[-1] in TADILAT):
        alanlar = alanlar[:-1]
    disiplinler = {x["code"]: x for x in d["disciplines"]}
    disiplin = disiplinler.get(alanlar[0])
    if disiplin is None:
        return f"“{alanlar[0]}” EK-2 Tablo 2.2'de bir disiplin kodu değil"
    sablon = disiplin.get("layerTemplate")
    if sablon == "T4.3":
        if len(alanlar) not in (3, 4) or len(alanlar[1]) != 4 or len(alanlar[2]) != 4:
            return "peyzaj katmanı disiplin, alt disiplin (4), öge (4) ve isteğe bağlı malzeme olmalı (Tablo 4.3)"
        malzeme = alanlar[3] if len(alanlar) == 4 else ""
    else:
        if len(alanlar) != 3:
            return "disiplin, öge ya da alt disiplin ve malzeme: üç alan olmalı (Tablo 4.1–4.2)"
        if len(alanlar[1]) != 4 or not alanlar[1].isalpha():
            return f"ikinci alan “{alanlar[1]}” dört harf olmalı"
        malzeme = alanlar[2]
    if len(malzeme) > 15:
        return f"malzeme alanı “{malzeme}” 15 karakteri geçiyor (4.10)"
    return None


def _katmanlar(cad: dict, d: dict) -> list[dict]:
    out: list[dict] = []
    katmanlar = cad.get("layers") or []
    aks = {g["layer"] for g in cad.get("grids") or []}
    for k in katmanlar:
        if not k.get("objects"):
            continue
        ad = k["name"]
        uids = k.get("first") or []
        if ad.strip() == "0":
            out.append(_bulgu(
                d, "layer_zero", "digital_ek4_layer_zero",
                f"“0” katmanında {k['objects']} nesne var",
                "Nesneleri türlerinin EK-4 katmanına taşıyın; “0” katmanında çizim yapılmaz.",
                uids, None))
            continue
        neden = _katman_adi(ad, d)
        if neden:
            oneri = _ascii_buyuk(ad.strip()).replace(" ", "")
            fix = None
            if oneri != ad and _katman_adi(oneri, d) is None and oneri not in {x["name"] for x in katmanlar}:
                fix = {"label": f"Katmanın adını {oneri} yap", "command": "cad.layers.set",
                       "input": {"operation": "rename", "layer": k["id"], "name": oneri}}
            out.append(_bulgu(
                d, "layer_name", "digital_ek4_layer_name",
                f"“{ad}” katmanı EK-4 adı değil: {neden}",
                (f"Katmanın adını {oneri} yapın." if fix else
                 "Katmanı disiplin, öge ve malzemeyle adlandırın (MM-DUVA-TUGLA) ya da nesnelerini EK-4 katmanına "
                 "taşıyın; Mimari'de katman anahtarı nesneyi türünün katmanına yazar."),
                uids, None, fix=fix))
        else:
            alanlar = ad.split("-")
            if len(alanlar) >= 3 and alanlar[1] == "EMSL" and alanlar[2] not in d["emsal"]:
                out.append(_bulgu(
                    d, "emsal_layer", "digital_ek4_emsal_layer",
                    f"“{ad}” emsal katmanının malzeme alanı {alanlar[2]}: DAHIL, HARIC ya da DIGER olmalı",
                    "Emsal katmanını MM-EMSL-DAHIL, MM-EMSL-HARIC ya da MM-EMSL-DIGER yapın.",
                    uids, None))
        if k.get("hatches") and not ad.endswith("-T"):
            out.append(_bulgu(
                d, "hatch_layer", "digital_ek4_hatch_layer",
                f"“{ad}” katmanında {k['hatches']} tarama var; tarama adının sonu “-T” olan ayrı katmanda olmalı",
                f"Taramaları {ad}-T katmanına taşıyın.",
                k.get("firstHatches") or uids, None))
        if k["id"] in aks and k.get("lineType") != "dashdot":
            cizgi = _CIZGI.get(k.get("lineType") or "", k.get("lineType") or "bilinmeyen")
            out.append(_bulgu(
                d, "axis_line", "digital_ek4_axis_line",
                f"Aksların katmanı “{ad}” {cizgi} çizgiyle çiziliyor; aks çizgisi noktalı kesikli olmalı",
                "Katmanın çizgi tipini noktalı kesikli yapın.",
                [g["uid"] for g in cad.get("grids") or [] if g["layer"] == k["id"]], None,
                fix={"label": "Çizgi tipini noktalı kesikli yap", "command": "cad.layers.edit",
                     "input": {"operation": "style", "layer": k["id"], "lineType": "dashdot"}}))
    return out


# --- EK-4: ölçü yazısı ve bloklar ---------------------------------------------------------------------------------


def _olcu(cad: dict, d: dict) -> list[dict]:
    out: list[dict] = []
    izinli = {_katla(f) for f in d["dimensionFonts"]}
    for f in cad.get("dimensionFonts") or []:
        font = f["font"]
        if _katla(font) in izinli or _katla(font.removesuffix(".shx")) in izinli:
            continue
        if font == "Arimo":
            out.append(_bulgu(
                d, "dimension_font", "digital_ek4_dimension_font",
                f"{f['count']} ölçünün yazısı Arimo: Arial'ın ölçüleriyle aynı açık yazı tipi; 4.21.2 Arial der, "
                f"idarenin teyidi gerekir (K29)",
                "İdare Arimo'yu kabul etmiyorsa ölçülerin yazı tipini Arial'a çevirin (DXF'te zaten arial.ttf yazılır).",
                [], None, onem="info"))
            continue
        out.append(_bulgu(
            d, "dimension_font", "digital_ek4_dimension_font",
            f"{f['count']} ölçünün yazısı {font}: ölçü yazısı Times New Roman, Arial, Calibri, Romans ya da Tahoma "
            f"olmalı",
            "Çizimin yazı tipini Proje ayarları'nda Arimo (Arial ölçülerinde) yapın ya da ölçülerin yazı tipini değiştirin.",
            [], None))
    return out


def _bloklar(cad: dict, d: dict) -> list[dict]:
    out: list[dict] = []
    disiplinler = {x["code"] for x in d["disciplines"]}
    for b in cad.get("blocks") or []:
        ad = b["name"]
        alanlar = ad.split("-")
        neden = None
        if ad != _ascii_buyuk(ad) or " " in ad:
            neden = "büyük harf, Türkçe harfsiz ve boşluksuz olmalı"
        elif any(not a or not _ALAN.match(a) for a in alanlar):
            neden = "alanlar orta tireyle ayrılmış harf ve rakam olmalı"
        elif alanlar[0] not in disiplinler:
            neden = f"“{alanlar[0]}” bir disiplin kodu değil"
        elif not 2 <= len(alanlar) <= 4 or len(alanlar[1]) != 4:
            neden = "disiplin, öge (4), isteğe bağlı malzeme ve ebat olmalı"
        elif len(alanlar) >= 3 and len(alanlar[2]) > 15:
            neden = "malzeme alanı 15 karakteri geçiyor"
        elif len(alanlar) == 4 and len(alanlar[3]) > 10:
            neden = "ebat alanı 10 karakteri geçiyor"
        if neden:
            out.append(_bulgu(
                d, "block_name", "digital_ek4_block_name",
                f"“{ad}” bloğu ({b['inserts']} kez yerleştirilmiş) EK-4 Tablo 4.4'ün adı değil: {neden}",
                "Bloğu disiplin, öge ve isteğe bağlı malzeme ve ebatla adlandırın (MM-TEFR-YATAK-160).",
                [], None))
    return out
