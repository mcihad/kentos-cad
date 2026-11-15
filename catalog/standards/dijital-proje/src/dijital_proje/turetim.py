"""Paftanın modelden üretildiği (docs/adr/8065; plan/bim-plan.md BIM-04; Dijital Hazırlama Yönetmeliği Md. 9/1: "CAD çizim
dosyaları, BIM modelinden üretilir"; EK-9 Tablo 9.1 satır 21: "Teslim edilen PDF paftaları BIM modelinden üretildi mi?").

Kural çizimi okumaz: KentOS'un ölçtüğü olgulara (``facts.project.cad``) bakar. Bulgularının hepsi ``error``'dır: ruhsat yayını
onlar kapanmadan yapılmaz (Ruhsat takımı › Paftaları toplu yayınla).

- ``derived_stale`` — kesit ya da görünüş eski: model, çıkarıldığından beri değişti. Düzeltmesi Yeniden çiz, tek adım.
- ``derived_gone`` — çizimi duran kesit ya da görünüşün işareti silinmiş: kaynağı yok.
- ``free_on_model_layer`` — model nesnesi taşıyan katmanda serbest çizim (çizgi, yay, tarama …): modelden gelmeyen çizim
  modelin katmanında durmaz; detay çizimi kendi katmanındadır.
- ``annotation_broken`` — nesnesi silinmiş etiket ya da bağı kopmuş ölçü: paftada modelin bugünkü hâlini söylemez.

Hesap tabloları, mahal listesi ve doğrama listesi canlıdır: çizim değiştiği adımda yenilenir, eskimez.
"""

from __future__ import annotations

from typing import Any

from . import ek34

#: The article and the gazette every finding names.
MADDE = "Dijital Proje Yön. 9"
RG = "RG 5.8.2026/33331"

#: A sheet's derived drawing as a finding says it.
_TUR = {"section": "kesit", "elevation": "görünüş"}


def denetle(facts: dict[str, Any], data: dict[str, Any] | None = None) -> list[dict[str, Any]]:
    """``facts``: ``kentos.auditFacts`` (``cad`` bölümü okunur). Bulgular ``ek34.denetle``'ninkiyle aynı biçimdedir."""
    d = data if data is not None else ek34.veri()
    cad = facts.get("cad") or {}
    out: list[dict[str, Any]] = []
    for t in cad.get("derived") or []:
        tur = _TUR.get(t.get("kind"), "çizim")
        if t.get("state") == "stale" and t.get("mark"):
            out.append(_bulgu(
                d, "derived_stale", "digital_derived_stale",
                f"{t['name']} eski: model, {tur} çıkarıldığından beri değişti; paftada eski {tur} basılır",
                f"Öznitelikler › {t['name']} › Durum'da Yeniden çiz ile güncelleyin.",
                [t["mark"]],
                fix={"label": f"{t['name']} yeniden çiz", "command": "cad.sections.edit",
                     "input": {"operation": {"kind": "refresh", "mark": t["mark"]}}}))
        elif t.get("state") == "gone" and t.get("inserts"):
            out.append(_bulgu(
                d, "derived_gone", "digital_derived_gone",
                f"{t['name']}: işareti silinmiş, çizimin kaynağı yok",
                f"{tur.capitalize()} işaretini geri getirin ya da yerleşimlerini silin; modelden gelmeyen {tur} paftaya girmez.",
                []))
    for k in cad.get("layers") or []:
        if k.get("model") and k.get("free"):
            out.append(_bulgu(
                d, "free_on_model_layer", "digital_free_on_model_layer",
                f"“{k['name']}” katmanında {k['free']} serbest çizim var; bu katman modelin ({k['model']} nesne)",
                "Serbest çizimi detay katmanına taşıyın ya da modelin nesnesiyle yeniden çizin; modelden gelmeyen çizim "
                "modelin katmanında durmaz.",
                k.get("firstFree") or []))
    kirik = cad.get("broken") or []
    for b in kirik:
        ne = "etiketin nesnesi silinmiş" if b.get("kind") == "tag" else "ölçünün nesneye bağı kopmuş"
        out.append(_bulgu(
            d, "annotation_broken", "digital_annotation_broken",
            f"Paftadaki bir {ne}: modelin bugünkü hâlini söylemiyor",
            "Denetim › Etiketler ya da Ölçüler'deki bulgusunu düzeltin.",
            [b["uid"]]))
    return out


def _bulgu(d: dict, kural: str, kod: str, mesaj: str, onar: str, uids: list[str], fix=None) -> dict[str, Any]:
    return {
        "rule": kural,
        "code": kod,
        "severity": "error",
        "message": f"{mesaj} — {MADDE}",
        "repair": onar,
        "uids": list(uids),
        "at": [0.0, 0.0],
        "cite": " · ".join([MADDE, RG, *d["rules"][kural]]),
        "fix": fix,
    }
