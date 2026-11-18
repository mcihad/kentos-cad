"""Acil çıkış beyanı ve kaçış yolları (docs/adr/8124; plan/bim-plan.md BIM-07): kapının ve merdivenin acil çıkış rolü
(EK-6 Tablo 6.25 ``Pset_DoorCommon.FireExit``, Tablo 6.61 ``Pset_StairCommon.FireExit``) müellifin beyanıdır. KentOS'un
kaçış yolları (ADR 8035) bir kapıyı ya da merdiveni kat çıkışı sayıyorsa öneri evet, saymıyorsa hayırdır. Kural modeli
okumaz: KentOS'un ölçtüğü olgulara (``facts.cad.exits``: her kapı ve merdiven, beyanı ve yolların onu neden çıkış
saydığı) bakar.

- ``exit_suggestion`` — beyanı olmayan kapılar ve merdivenler: yolların önerisi, adaylar adıyla; düzeltmesi önerinin
  hepsini tek adımda yazar (bilgi). Müellif okur ve onaylar; eksik ``FireExit`` ayrıca ``model_required``'dadır.
- ``exit_mismatch`` — beyan ile yol uyuşmuyor: evet denmiş ama yollar onu çıkış saymıyor, ya da hayır denmiş ama yollar
  onu çıkış sayıyor (uyarı). Düzeltmesi önerinin yönündedir; yolların görmediği bir çıkış (dış merdiven, çizilmemiş
  mahal) olabilir, karar müellifindir.
"""

from __future__ import annotations

from typing import Any

RG = "RG 5.8.2026/33331"
KOMUT = "cad.arch.performance"

#: Why the routes take it, as the facts' key says it.
_NEDEN = {
    "outside": "dışarıya açılan kapı, binanın son çıkışı",
    "intoStair": "kaçış merdiveninin mahaline açılan kapı, kat çıkışı",
    "openStair": "ortak mahalde duran açık merdiven, kat çıkışı",
    "stairRoom": "kaçış merdiveninin mahalindeki merdiven",
}


def _ad(x: dict[str, Any]) -> str:
    tur = "Kapı" if x["kind"] == "door" else "Merdiven"
    return f"{tur} {x['name']}" if x.get("name") else tur


def _sayi(kapi: int, merdiven: int, kapilar: str, merdivenler: str, hic: str) -> str:
    """“2 kapıyı ve 1 merdiveni”; sıfır olan söylenmez, ikisi de sıfırsa ``hic``."""
    parcalar = [f"{n} {t}" for n, t in ((kapi, kapilar), (merdiven, merdivenler)) if n]
    return " ve ".join(parcalar) or hic


def _cite(x: dict[str, Any] | None = None) -> str:
    tablo = "Tablo 6.25 ve 6.61" if x is None else ("Tablo 6.25" if x["kind"] == "door" else "Tablo 6.61")
    return f"EK-6 {tablo} (FireExit) · BYKHY 4/u (kat çıkışı) · {RG}"


def _fix(evet: list[str], hayir: list[str], label: str) -> dict[str, Any]:
    return {"label": label, "command": KOMUT, "input": {"operation": {"kind": "exits", "yes": evet, "no": hayir}}}


def denetle(facts: dict[str, Any], data: dict[str, Any] | None = None) -> list[dict[str, Any]]:
    """``facts``: ``{"cad": {"exits": [{uid, kind, level, name, declared, route}]}}``."""
    cikislar = (facts.get("cad") or {}).get("exits") or []
    out: list[dict[str, Any]] = []
    bos = [x for x in cikislar if x.get("declared") is None]
    if bos:
        evet = [x for x in bos if x.get("route")]
        hayir = [x for x in bos if not x.get("route")]
        kapi = lambda xs: sum(1 for x in xs if x["kind"] == "door")  # noqa: E731
        merdiven = lambda xs: sum(1 for x in xs if x["kind"] == "stair")  # noqa: E731
        adaylar = ", ".join(f"{_ad(x)} ({_NEDEN.get(x['route'], x['route'])})" for x in evet[:8])
        if len(evet) > 8:
            adaylar += f" ve {len(evet) - 8} tane daha"
        ne = (f"Acil çıkış önerisi: kaçış yolları "
              f"{_sayi(kapi(evet), merdiven(evet), 'kapıyı', 'merdiveni', 'hiçbir kapıyı ya da merdiveni')} çıkış "
              f"sayıyor; {_sayi(kapi(hayir), merdiven(hayir), 'kapı', 'merdiven', 'öbürü yok')}"
              + (" çıkış değil" if hayir else ""))
        if adaylar:
            ne += f". Adaylar: {adaylar}"
        out.append({
            "rule": "exit_suggestion",
            "code": "digital_exit_suggestion",
            "severity": "info",
            "message": f"{ne} — beyanı olmayan {len(bos)} nesne",
            "repair": "Adayları okuyun; doğruysa düzeltme önerinin hepsini müellif beyanı olarak tek adımda yazar "
                      "(Öznitelikler › Acil çıkış'tan tek tek de değişir).",
            "uids": [x["uid"] for x in evet] + [x["uid"] for x in hayir],
            "at": [0.0, 0.0],
            "cite": _cite(),
            "fix": _fix([x["uid"] for x in evet], [x["uid"] for x in hayir],
                        f"Önerileri yaz ({len(evet)} evet, {len(hayir)} hayır)"),
        })
    for x in cikislar:
        beyan, yol = x.get("declared"), x.get("route")
        if beyan is None or beyan == bool(yol):
            continue
        if beyan:
            ne = f"{_ad(x)}: acil çıkış diye beyan edilmiş ama kaçış yolları onu çıkış saymıyor"
            onarim = ("Dışarıya ya da kaçış merdiveninin mahaline açılmıyorsa beyanı hayır yapın; yolların görmediği bir "
                      "çıkışsa (dış merdiven, çizilmemiş mahal) mahalleri ve merdiveni çizin.")
            fix = _fix([], [x["uid"]], "Acil çıkış: hayır")
        else:
            ne = (f"{_ad(x)}: acil çıkış değil diye beyan edilmiş ama kaçış yolları onu çıkış sayıyor "
                  f"({_NEDEN.get(yol, yol)})")
            onarim = "Kapı ya da merdiven yangında çıkış olarak kullanılacaksa beyanı evet yapın."
            fix = _fix([x["uid"]], [], "Acil çıkış: evet")
        out.append({
            "rule": "exit_mismatch",
            "code": "digital_exit_mismatch",
            "severity": "warning",
            "message": f"{ne} — Dijital Proje Yön. EK-6",
            "repair": onarim,
            "uids": [x["uid"]],
            "at": [0.0, 0.0],
            "cite": _cite(x),
            "fix": fix,
        })
    return out
