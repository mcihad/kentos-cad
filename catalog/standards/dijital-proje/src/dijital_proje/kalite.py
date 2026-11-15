"""EK-9 Model Kalite Kontrol Formu'nun modelde ölçülen soruları (docs/adr/8067; plan/bim-plan.md BIM-06). Kural modeli
okumaz: KentOS'un ölçtüğü olgulara (``facts.cad``) bakar.

- ``quality_room_overlap`` — Tablo 9.1 satır 13: "Mahaller diğer mahallerle örtüşmeyecek şekilde ve doğru katta modellendi
  mi?" Aynı kattaki iki mahal (ya da ikisi de katsız) aynı zemini kaplıyorsa hata. Ortak alan KentOS'un ölçtüğüdür
  (``cad.roomOverlaps``: iki mahalin çizilen alanlarının kesişimi, 0,0001 m²'ye); bir eşiğin altındaki ortak alan
  çizimin yuvarlamasıdır, örtüşme sayılmaz.
"""

from __future__ import annotations

from typing import Any

RG = "RG 5.8.2026/33331"
SATIR_13 = "EK-9 Tablo 9.1 satır 13 · PDF s. 239"

#: Bundan küçük ortak alan iki mahalin aynı zemini kapladığı değil, çizimin yuvarlamasıdır (m²): 10 cm'lik bir kare.
ESIK = 0.01


def denetle(facts: dict[str, Any], data: dict[str, Any] | None = None) -> list[dict[str, Any]]:
    """``facts``: ``{"cad": {"rooms": [{uid, number, name, levelCode}], "roomOverlaps": [{a, b, area, at}]}}``."""
    cad = facts.get("cad") or {}
    mahaller = {r["uid"]: r for r in cad.get("rooms") or []}
    out: list[dict[str, Any]] = []
    for o in cad.get("roomOverlaps") or []:
        if float(o.get("area") or 0.0) < ESIK:
            continue
        a, b = mahaller.get(o["a"], {}), mahaller.get(o["b"], {})
        kat = a.get("levelCode") or b.get("levelCode")
        yer = f" ({kat} katında)" if kat else ""
        out.append({
            "rule": "quality_room_overlap",
            "code": "digital_quality_room_overlap",
            "severity": "error",
            "message": f"{_ad(a, o['a'])} ile {_ad(b, o['b'])}{yer} {float(o['area']):.2f} m² örtüşüyor: "
                       "mahaller birbirini kaplamaz — Dijital Proje Yön. EK-9/13",
            "repair": "İki mahalden birinin sınırını ortak duvarın yüzüne çekin ya da fazla olan mahali silin; "
                      "Toplu mahal bul sınırları duvarlardan yeniden bulur.",
            "uids": [o["a"], o["b"]],
            "at": list(o.get("at") or [0.0, 0.0]),
            "cite": " · ".join([SATIR_13, RG]),
            "fix": None,
        })
    return out


def _ad(mahal: dict[str, Any], uid: str) -> str:
    """Mahalin numarası ve adı, yoksa kimliğinin başı: bulguda okunacak ad."""
    parca = [x for x in (mahal.get("number"), mahal.get("name")) if x]
    return f"“{' '.join(parca)}”" if parca else f"“{uid[:8]}” mahali"
