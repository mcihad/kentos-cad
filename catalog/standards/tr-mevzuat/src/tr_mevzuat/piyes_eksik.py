"""Konutta eksik piyes (PAİY 29/1), yönetmeliğin kendi istisnalarıyla.

- 29/1: “Her müstakil konutta en az aşağıdaki piyesler bulunur”: oturma odası, yatak odası, mutfak veya yemek pişirme
  yeri, banyo veya yıkanma yeri, tuvalet.
- 29/2: “3 veya daha az odalı konutlarda banyo/yıkanma yeri ile tuvalet aynı yerde düzenlenebilir.” Oda sayısı oturma
  ve yatak odalarıdır (KentOS Mimari'nin yerel denetimi de böyle sayar).
- 29/4: “Mutfak nişi ve oda ile banyo ve tuvaletin aynı mekânda düzenlenmesi halinde her mekân için öngörülen en az
  alanların toplamı kadar alan düzenlenmek zorundadır.” Mutfak nişi olan oda kendi en az alanı ile mutfağın 3.30
  m²'sini, tuvaleti de karşılayan banyo 3.00 + 1.20 = 4.20 m²'yi taşır.
- 5/25 (ek cümle, RG 13.8.2025/32985): “Parseldeki bağımsız bölüm sayısının %20’sinden fazla olmamak üzere
  oluşturulacak bağımsız bölümlerde; oturma odası dahilinde dar kenarı 2.50 metreden ve alanı 9 m2’den az olmamak
  üzere yatak nişi teşkil edilmesi halinde ayrıca yatak odası yapılmayabilir.”

Olgular nişi, mahalin donanımını ve parseldeki payı görmez: istisna mümkünse bulgu bilgidir ("karar verilemedi"),
mümkün değilse hatadır. Adından türü okunamayan bir mahal eksik türün en az ölçüsünü karşılıyorsa o tür olabilir:
karar verilemez, mahalin adı düzeltilmeli; karşılamıyorsa piyes ya yoktur ya ölçüsü yetmez, ikisi de aykırıdır.
"""

from __future__ import annotations

from collections.abc import Callable

from kentos import ext

from .bina import PAY, bolum, mahal, metre, metrekare

#: PAİY 29/1: piyesin en az dar kenarı (m) ve net alanı (m²), tablonun sırasıyla.
OLCU: dict[str, tuple[float, float]] = {
    "living": (3.00, 12.0),
    "bedroom": (2.50, 9.00),
    "kitchen": (1.50, 3.30),
    "bath": (1.50, 3.00),
    "toilet": (1.00, 1.20),
}
AD = {"living": "oturma odası", "bedroom": "yatak odası", "kitchen": "mutfak", "bath": "banyo", "toilet": "tuvalet"}


def madde(*fikralar: str) -> str:
    """Bulgunun söylediği madde ve fıkralar: “PAİY md. 29/2, 29/4 (RG 3.7.2017/30113)”."""
    return f"PAİY md. {', '.join(fikralar)} (RG 3.7.2017/30113)"


M29_1 = madde("29/1")
#: 29/4'ün toplamları: tuvaleti de karşılayan banyo; mutfak nişi olan oda kendi alanına mutfağınkini ekler.
BANYO_TUVALET = OLCU["bath"][1] + OLCU["toilet"][1]
NIS = OLCU["kitchen"][1]


def _olabilir(r: ext.RoomFacts, kind: str) -> bool:
    """Türü adından okunamayan mahal bu tür olabilir mi: ölçüsü türün en azını karşılıyor (sınırı bulunamayanın ölçüsü
    bilinmez, olabilir)."""
    if r.broken:
        return True
    narrow, area = OLCU[kind]
    return r.net >= area - PAY and (r.narrow is None or r.narrow >= narrow - PAY)


def _bulgu(severity: str, unit: ext.UnitFacts, text: str) -> ext.AuditFinding:
    return ext.AuditFinding(message=f"{bolum(unit)}: {text}", severity=severity)


def _belki(unit: ext.UnitFacts, kind: str, maybe: list[ext.RoomFacts]) -> ext.AuditFinding:
    narrow, area = OLCU[kind]
    names = ", ".join(mahal(r) for r in maybe[:3]) + (" …" if len(maybe) > 3 else "")
    return _bulgu(
        "info",
        unit,
        f"adından {AD[kind]} olduğu okunan piyes yok; türü adından okunamayan {names} {AD[kind]} olabilir (en az "
        f"{metrekare(area)}, dar kenarı {metre(narrow)}), karar verilemedi: mahalin adını düzeltin — {M29_1}.",
    )


Rooms = dict[str, list[ext.RoomFacts]]


def _oturma(unit: ext.UnitFacts, by: Rooms, odalar: int, maybe: list[ext.RoomFacts]) -> ext.AuditFinding | None:
    if maybe:
        return _belki(unit, "living", maybe)
    return _bulgu(
        "error",
        unit,
        f"oturma odası yok; her konutta net alanı en az {metrekare(12.0)}, dar kenarı en az {metre(3.0)} bir oturma "
        f"odası bulunur ve yönetmelik bunun istisnasını saymaz — {madde('29/1', '5/25')}.",
    )


def _yatak(unit: ext.UnitFacts, by: Rooms, odalar: int, maybe: list[ext.RoomFacts]) -> ext.AuditFinding | None:
    if maybe:
        return _belki(unit, "bedroom", maybe)
    hosts = [r for r in by["living"] if _olabilir(r, "bedroom")]
    if hosts:
        return _bulgu(
            "info",
            unit,
            "yatak odası yok; parseldeki bağımsız bölümlerin %20’sini geçmeyenlerinde oturma odası içinde dar kenarı "
            "2.50 metreden ve alanı 9 m²'den az olmayan yatak nişi varsa ayrıca yatak odası yapılmayabilir; olgular "
            f"nişi ve parseldeki payı görmez, karar verilemedi — {madde('5/25', '29/1')}.",
        )
    where = "oturma odası da yok" if not by["living"] else "oturma odasında ona yer yok"
    return _bulgu(
        "error",
        unit,
        f"yatak odası yok; yatak nişi (dar kenarı 2.50 m, alanı 9 m²) oturma odasının içinde olur ve {where} — "
        f"{madde('29/1', '5/25')}.",
    )


def _mutfak(unit: ext.UnitFacts, by: Rooms, odalar: int, maybe: list[ext.RoomFacts]) -> ext.AuditFinding | None:
    if maybe:
        return _belki(unit, "kitchen", maybe)
    rooms = by["living"] + by["bedroom"]
    hosts = [r for r in rooms if r.broken or r.net >= OLCU[str(r.kind)][1] + NIS - PAY]
    sums = (
        f"oturma odasında {metrekare(OLCU['living'][1] + NIS)}, yatak odasında {metrekare(OLCU['bedroom'][1] + NIS)}"
    )
    if hosts:
        return _bulgu(
            "info",
            unit,
            f"mutfak yok; mutfak nişi bir odadaysa o oda iki piyesin en az alanlarının toplamı kadar olur ({sums}) ve "
            f"{mahal(hosts[0])} buna yetiyor; olgular nişi görmez, karar verilemedi — {madde('29/1', '29/4')}.",
        )
    return _bulgu(
        "error",
        unit,
        f"mutfak yok; mutfak nişi bir odada olsaydı o oda iki piyesin en az alanlarının toplamı kadar olurdu ({sums}), "
        f"hiçbir oda buna yetmiyor — {madde('29/1', '29/4')}.",
    )


def _banyo(unit: ext.UnitFacts, by: Rooms, odalar: int, maybe: list[ext.RoomFacts]) -> ext.AuditFinding | None:
    if maybe:
        return _belki(unit, "bath", maybe)
    if odalar <= 3:
        combined = [
            r
            for r in by["toilet"]
            if r.broken or (r.net >= BANYO_TUVALET - PAY and (r.narrow is None or r.narrow >= OLCU["bath"][0] - PAY))
        ]
        if combined:
            return _bulgu(
                "info",
                unit,
                f"banyo yok; 3 veya daha az odalı konutta banyo ile tuvalet aynı yerde düzenlenebilir, o yer iki "
                f"piyesin en az alanlarının toplamı kadar olur ({metrekare(BANYO_TUVALET)}) ve {mahal(combined[0])} "
                f"buna yetiyor; olgular mahalin donanımını görmez, karar verilemedi — {madde('29/2', '29/4')}.",
            )
    return _bulgu(
        "error",
        unit,
        f"banyo yok; her konutta net alanı en az {metrekare(3.0)}, dar kenarı en az {metre(1.5)} bir banyo veya "
        f"yıkanma yeri bulunur — {M29_1}.",
    )


def _tuvalet(unit: ext.UnitFacts, by: Rooms, odalar: int, maybe: list[ext.RoomFacts]) -> ext.AuditFinding | None:
    if odalar <= 3 and by["bath"]:
        measured = [r for r in by["bath"] if not r.broken]
        if any(r.net >= BANYO_TUVALET - PAY for r in measured):
            return None
        if maybe:
            return _belki(unit, "toilet", maybe)
        if len(measured) < len(by["bath"]):
            return _bulgu(
                "info",
                unit,
                "tuvalet yok ve banyonun sınırı bulunamadı; 3 veya daha az odalı konutta tuvalet banyoyla aynı yerde "
                "olabilir, o banyonun iki piyesin toplamına yetip yetmediğine karar verilemedi — "
                f"{madde('29/2', '29/4')}.",
            )
        largest = max(measured, key=lambda r: r.net)
        return _bulgu(
            "error",
            unit,
            f"tuvalet yok; 3 veya daha az odalı konutta tuvalet banyoyla aynı yerde düzenlenebilir, o yer iki piyesin "
            f"en az alanlarının toplamı kadar olur ({metrekare(BANYO_TUVALET)}), en büyük banyo {mahal(largest)} "
            f"{metrekare(largest.net)} — {madde('29/2', '29/4')}.",
        )
    if maybe:
        return _belki(unit, "toilet", maybe)
    why = (
        f"banyo ile tuvaletin aynı yerde olabilmesi 3 veya daha az odalı konut içindir, bu konutta {odalar} oda var"
        if by["bath"]
        else "banyo da yok"
    )
    return _bulgu(
        "error",
        unit,
        f"tuvalet yok; {why}; her konutta net alanı en az {metrekare(1.2)}, dar kenarı en az {metre(1.0)} bir tuvalet "
        f"bulunur — {madde('29/1', '29/2')}.",
    )


_KARAR: dict[str, Callable[[ext.UnitFacts, Rooms, int, list[ext.RoomFacts]], ext.AuditFinding | None]] = {
    "living": _oturma,
    "bedroom": _yatak,
    "kitchen": _mutfak,
    "bath": _banyo,
    "toilet": _tuvalet,
}


def eksikler(unit: ext.UnitFacts, rooms: list[ext.RoomFacts]) -> list[ext.AuditFinding]:
    """Konutun eksik piyesleri, tablonun sırasıyla: istisnası mümkün olan bilgi, olmayan hata."""
    by: Rooms = {k: [r for r in rooms if r.kind == k] for k in OLCU}
    unknown = [r for r in rooms if r.kind is None]
    odalar = len(by["living"]) + len(by["bedroom"])
    findings: list[ext.AuditFinding] = []
    for kind in OLCU:
        if by[kind]:
            continue
        found = _KARAR[kind](unit, by, odalar, [r for r in unknown if _olabilir(r, kind)])
        if found is not None:
            findings.append(found)
    return findings
