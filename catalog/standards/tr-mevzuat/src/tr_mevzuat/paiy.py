"""Planlı Alanlar İmar Yönetmeliği (PAİY, RG 3.7.2017/30113) kuralları.

Madde 29 (Yapı piyesleri ve ölçüleri), birinci fıkra: “Her müstakil konutta en az aşağıdaki piyesler bulunur”;
tabloda oturma odası dar kenarı 3.00 metre, net alanı 12.0 m²; yatak odası 2.50 metre, 9.00 m²; mutfak veya yemek
pişirme yeri 1.50 metre, 3.30 m²; banyo veya yıkanma yeri 1.50 metre, 3.00 m²; tuvalet 1.00 metre, 1.20 m². Üçüncü
fıkra: “Hol ve koridor genişlikleri 1.20 metreden az olamaz.” İkinci ve dördüncü fıkralar ile 5/25'in yatak nişi
eksik piyesin istisnalarıdır (``piyes_eksik``).

Mahalin türü ve dar kenarı KentOS Mimari'nin okuyuşudur (ADR 8057 §53): tür adından ya da kullanımından (Salon,
Yatak odası, Mutfak, Banyo, WC, Hol …), dar kenar net sınırın içine sığan en büyük dairenin çapı. Konutun hiçbir
piyesinin türü okunamıyorsa (adlar türü söylemiyor ya da olgular eski bir KentOS'tan) yalnız türden bağımsız kesin sonuç
denetlenir: net alanı 12.0 m²'ye ulaşan piyes yoksa oturma odası hangi mahal olursa olsun sağlanamaz.
"""

from __future__ import annotations

from kentos import ext

from .bina import PAY, bolum, kat_adlari, konutlar, mahal, metre, metrekare
from .piyes_eksik import AD, M29_1, OLCU, eksikler, madde

#: PAİY 29/3: hol ve koridorun en az genişliği, m.
KORIDOR = 1.20
M29_3 = madde("29/3")


def _olculer(unit: ext.UnitFacts, rooms: list[ext.RoomFacts]) -> list[ext.AuditFinding]:
    """Türü bilinen her piyesin net alanı ve dar kenarı tablonun en azına: biri yetmiyorsa hata, ikisini de söyler."""
    findings: list[ext.AuditFinding] = []
    for r in rooms:
        if r.kind not in OLCU:
            continue
        narrow_min, area_min = OLCU[r.kind]
        what = f"{bolum(unit)}: {mahal(r)} ({AD[r.kind]})"
        if r.broken:
            findings.append(
                ext.AuditFinding(
                    message=f"{what} mahalinin sınırı bulunamadı; alanı ve dar kenarı denetlenemedi — {M29_1}.",
                    severity="info",
                    feature_id=r.uid,
                )
            )
            continue
        small = r.net < area_min - PAY
        narrow = r.narrow is not None and r.narrow < narrow_min - PAY
        if not (small or narrow):
            continue
        width = metre(r.narrow) if r.narrow is not None else "ölçülemedi"
        if small and narrow:
            short = "alanı da dar kenarı da yetmiyor"
        else:
            short = "alanı yetmiyor" if small else "dar kenarı yetmiyor"
        findings.append(
            ext.AuditFinding(
                message=(
                    f"{what} net alanı {metrekare(r.net)} (en az {metrekare(area_min)}), dar kenarı {width} (en az "
                    f"{metre(narrow_min)}); {short} — {M29_1}."
                ),
                severity="error",
                feature_id=r.uid,
            )
        )
    return findings


def _turu_okunamayan(unit: ext.UnitFacts, rooms: list[ext.RoomFacts]) -> list[ext.AuditFinding]:
    """Piyeslerin türü okunamayan konut: net alanı 12.0 m²'ye ulaşan piyes yoksa oturma odası sağlanamaz (hata);
    sınırı bulunamayan mahali varsa ve ölçülenlerin hiçbiri yetmiyorsa karar verilemez (bilgi)."""
    measured = [r for r in rooms if not r.broken]
    largest = max(measured, key=lambda r: r.net, default=None)
    area = OLCU["living"][1]
    if largest is not None and largest.net >= area - PAY:
        return []
    broken = [r for r in rooms if r.broken]
    if broken:
        return [
            ext.AuditFinding(
                message=(
                    f"{bolum(unit)}: {mahal(broken[0])} mahalinin sınırı bulunamadı, alanı güvenilir değil; net alanı "
                    f"en az {metrekare(area)} olan bir oturma odası denetlenemedi — {M29_1}."
                ),
                severity="info",
                feature_id=broken[0].uid,
            )
        ]
    if largest is None:
        return []
    return [
        ext.AuditFinding(
            message=(
                f"{bolum(unit)}: piyeslerin türü adlarından okunamadı ve net alanı en az {metrekare(area)} olan piyes "
                f"yok (en büyüğü {mahal(largest)}, {metrekare(largest.net)}); her konutta net alanı en az "
                f"{metrekare(area)}, dar kenarı en az {metre(OLCU['living'][0])} bir oturma odası bulunur — {M29_1}."
            ),
            severity="error",
            feature_id=largest.uid,
        )
    ]


@ext.audit("piyesler")
def piyesler(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Her konut bağımsız bölümünün piyesleri (PAİY 29/1): türü bilinen her piyesin net alanı ve dar kenarı tablonun
    en azına (hata), eksik piyes 29/2, 29/4 ve 5/25'in istisnalarıyla (istisna mümkünse bilgi, değilse hata).
    Piyeslerinin hiçbirinin türü okunamayan konutta yalnız 12.0 m²'lik piyesin varlığı denetlenir. Boşluk, dış mahal
    ve kaçış merdiveni mahali piyes sayılmaz; eklentinin mahalleri konutun piyesi değildir."""
    a = facts.project.architecture
    findings: list[ext.AuditFinding] = []
    for unit_id, unit in konutlar(a).items():
        rooms = [r for r in a.rooms if r.unit == unit_id and not (r.void or r.outdoor or r.stair)]
        if not rooms:
            continue
        if not any(r.kind in OLCU for r in rooms):
            findings.extend(_turu_okunamayan(unit, rooms))
            continue
        findings.extend(_olculer(unit, rooms))
        findings.extend(eksikler(unit, rooms))
    return findings


@ext.audit("koridor")
def koridor(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Hol ve koridor (türü ``corridor``) en az 1.20 m genişlikte (PAİY 29/3): dar kenarı daha az olan hata. Konutun
    ve ortak alanın holleri ile bir bölüme ait olmayanlar okunur; dükkânın, büronun ya da eklentinin kendi holü
    okunmaz (KentOS Mimari'nin yerel denetimi gibi). Sınırı bulunamayan mahal denetlenmez."""
    a = facts.project.architecture
    units = {u.id: u for u in a.units}
    levels = kat_adlari(a)
    findings: list[ext.AuditFinding] = []
    for r in a.rooms:
        if r.kind != "corridor" or r.void or r.outdoor or r.stair or r.broken or r.narrow is None:
            continue
        unit = units.get(r.unit) if r.unit is not None else None
        if unit is not None and unit.kind in ("unit", "annex") and not unit.dwelling:
            continue
        if r.narrow >= KORIDOR - PAY:
            continue
        findings.append(
            ext.AuditFinding(
                message=(
                    f"{mahal(r)} ({levels.get(r.level, r.level)}): net genişliği {metre(r.narrow)}; “Hol ve koridor "
                    f"genişlikleri 1.20 metreden az olamaz.” — {M29_3}."
                ),
                severity="error",
                feature_id=r.uid,
            )
        )
    return findings
