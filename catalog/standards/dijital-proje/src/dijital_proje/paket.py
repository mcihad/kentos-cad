"""2B ruhsat paketi (docs/adr/8066; plan/bim-plan.md BIM-05): yayınlanacak paftaların dosyaları yönetmeliğe uyuyor mu.
Dayanak: Md. 4/2 (2B çizim dosyaları PDF/A), Md. 6 ve EK-2 (dosya adları), Md. 8 (A4 ve katları; her çizim ayrı dosya).

Kural paketi üretmez, okur: KentOS'un yayın denetiminin söylediği dosyaları, paftaları ve seçenekleri (``facts.package``).
Bulguların hepsi ``error``'dır: ruhsat yayınının kapısı onlar kapanmadan açılmaz.

- ``package_pdfa`` — PDF/A seçilmemiş. Bölüm ve düzey kurum profilinin satırıdır (K13; KentOS: PDF/A-2b).
- ``package_one_sheet`` — bir dosyada birden çok pafta.
- ``package_paper`` — A4 ve katları dışında kâğıt: her kenar 210 ya da 297 mm'nin bir katı (±1,5 mm), en az A4 (K39).
- ``package_name`` — dosya adı EK-2 Tablo 2.1'in dokuz alanı değil: alanlar "_" ile, uzunlukları ve türleri tablodaki gibi;
  disiplin Tablo 2.2'den, dosya türü 2.12'den, dosya tipi 2.13'ten.
"""

from __future__ import annotations

import re
from typing import Any

from . import ek34

RG = "RG 5.8.2026/33331"

#: Every side a multiple of these, mm (A4's sides); the tolerance of a printed size.
_A4 = (210.0, 297.0)
_PAY = 1.5


def denetle(facts: dict[str, Any], data: dict[str, Any] | None = None) -> list[dict[str, Any]]:
    """``facts``: ``{"package": {"pdfa", "mode", "files": [{name, sheets}], "sheets": [{id, number, name, paper}]}}``."""
    d = data if data is not None else ek34.veri()
    paket = facts.get("package") or {}
    if not paket:
        return []
    out: list[dict[str, Any]] = []
    if not paket.get("pdfa"):
        out.append(_bulgu(d, "package_pdfa", "Dijital Proje Yön. 4/2", "digital_package_pdfa",
                          "PDF/A seçilmemiş: 2B çizim dosyaları PDF/A teslim edilir",
                          "Yayın penceresinde PDF/A-2 arşiv biçimini açın (kurumun istediği bölüm ve düzey: K13).", []))
    paftalar = {p["id"]: p for p in paket.get("sheets") or []}
    for f in paket.get("files") or []:
        if len(f.get("sheets") or []) > 1:
            out.append(_bulgu(d, "package_one_sheet", "Dijital Proje Yön. 8", "digital_package_one_sheet",
                              f"“{f['name']}” dosyasında {len(f['sheets'])} pafta var: her çizim ayrı dosyadır",
                              "Yayın penceresinde “Her pafta ayrı PDF”yi açın.", f["sheets"]))
        neden = _ad(f["name"], d)
        if neden:
            out.append(_bulgu(d, "package_name", "Dijital Proje Yön. 6", "digital_package_name",
                              f"“{f['name']}” EK-2 adı değil: {neden}",
                              "Dosya adını Ruhsat takımının adlandırmasına döndürün; künyenin proje kodunu, bina kodunu, "
                              "dosya türünü ve tipini, kat ya da konumu, sıra ve revizyon numarasını verin.",
                              f.get("sheets") or []))
    for p in paftalar.values():
        w, h = (float(x) for x in p.get("paper") or (0, 0))
        if not (_kat(w) and _kat(h) and min(w, h) >= 210 - _PAY and max(w, h) >= 297 - _PAY):
            ad = p.get("number") or p.get("name") or p["id"]
            out.append(_bulgu(d, "package_paper", "Dijital Proje Yön. 8", "digital_package_paper",
                              f"{ad} paftasının kâğıdı {w:.0f}×{h:.0f} mm: A4 ve katları değil",
                              "Paftanın kâğıdını A serisinden seçin (A4, A3, A2, A1, A0 ya da A4'ün katı bir uzun boy).",
                              [p["id"]]))
    return out


def _kat(kenar: float) -> bool:
    """Kenar 210 ya da 297 mm'nin bir katı mı (±1,5 mm)."""
    return any(abs(kenar - round(kenar / a) * a) <= _PAY and round(kenar / a) >= 1 for a in _A4)


def _ad(ad: str, d: dict) -> str | None:
    """Dosya adının EK-2'ye uymayan yanı; uyuyorsa ``None``."""
    # Uzantı alanlardan değildir: PDF de, BIM paketinin modeli de (.ifc, docs/adr/8075) aynı şablondadır.
    govde = re.sub(r"\.[A-Za-z0-9]+$", "", ad)
    alanlar = govde.split("_")
    sablon = d["fileTemplate"]
    if len(alanlar) != len(sablon):
        return f"{len(sablon)} alan alt tireyle ayrılmalı, {len(alanlar)} alan var"
    disiplinler = {x["code"] for x in d["disciplines"]}
    for i, (alan, s) in enumerate(zip(alanlar, sablon)):
        if len(alan) != s["length"]:
            return f"{s['name']} “{alan}” {s['length']} karakter olmalı"
        desen = {"Alfabetik": r"[A-Z]+", "Sayısal": r"[0-9]+"}.get(s["kind"], r"[A-Z0-9]+")
        if not re.fullmatch(desen, alan):
            return f"{s['name']} “{alan}” {s['kind'].lower()} ve büyük harf olmalı"
        if i == 2 and alan not in disiplinler:
            return f"“{alan}” EK-2 Tablo 2.2'de bir disiplin kodu değil"
        if i == 4 and alan not in d["fileKinds"]:
            return f"“{alan}” EK-2 Tablo 2.12'de bir dosya türü değil"
        if i == 5 and alan not in d["fileTypes"]:
            return f"“{alan}” EK-2 Tablo 2.13'te bir dosya tipi değil"
    return None


def _bulgu(d: dict, kural: str, madde: str, kod: str, mesaj: str, onar: str, uids: list[str]) -> dict[str, Any]:
    return {
        "rule": kural,
        "code": kod,
        "severity": "error",
        "message": f"{mesaj} — {madde}",
        "repair": onar,
        "uids": list(uids),
        "at": [0.0, 0.0],
        "cite": " · ".join([madde, RG, *d["rules"][kural]]),
        "fix": None,
    }
