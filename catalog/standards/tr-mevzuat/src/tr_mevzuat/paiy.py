"""Planlı Alanlar İmar Yönetmeliği (PAİY, RG 3.7.2017/30113) kuralları.

Madde 29 (Yapı piyesleri ve ölçüleri), birinci fıkra: “Her müstakil konutta en az aşağıdaki piyesler bulunur”;
tabloda oturma odası dar kenarı 3.00 metre, net alanı 12.0 m²; yatak odası 2.50 m, 9.00 m²; mutfak veya yemek pişirme
yeri 1.50 m, 3.30 m²; banyo veya yıkanma yeri 1.50 m, 3.00 m²; tuvalet 1.00 m, 1.20 m². Madde 5/25 aynı beş piyesi
sayar ve (RG 13.8.2025/32985 ek cümlesi) parseldeki bağımsız bölümlerin %20'sini geçmeyenlerinde oturma odasında
yatak nişi varsa ayrı yatak odası aramaz. Madde 29/4: mutfak nişi ve oda ile banyo ve tuvalet aynı mekânda ise o
mekân her birinin en az alanlarının toplamı kadar olur.

Olgular mahallerin türünü (oturma odası, yatak odası …) ve dar kenarını vermez; mahalin adı serbest yazıdır ve ondan
tür çıkarılmaz. Bu yüzden kural tablonun yalnız olgularla kesin karar verilen sonucunu denetler: konutta net alanı
12.0 m²'ye ulaşan hiçbir piyes yoksa oturma odası kuralı hangi mahal oturma odası olursa olsun sağlanamaz (yatak nişi
istisnası oturma odasını kaldırmaz; mutfak nişiyle birleşen oda daha da büyük olmalıdır).
"""

from __future__ import annotations

from kentos import ext

from .bina import PAY, bolum, konutlar, mahal, metrekare

#: PAİY 29/1: oturma odasının en az net alanı, m².
OTURMA_ODASI = 12.0
MADDE = "PAİY md. 29/1 (RG 3.7.2017/30113)"


@ext.audit("oturma-odasi")
def oturma_odasi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Her konut bağımsız bölümünde net alanı en az 12.0 m² olan bir piyes (oturma odasının en az alanı) bulunur;
    yoksa hata. Sınırı bulunamayan mahali olan bölümde ölçülen piyeslerin hiçbiri yetmiyorsa bilgi: alanı güvenilir
    değil, karar verilemedi. Boşluk, dış mahal ve kaçış merdiveni mahali piyes sayılmaz."""
    a = facts.project.architecture
    findings: list[ext.AuditFinding] = []
    for unit_id, unit in konutlar(a).items():
        rooms = [r for r in a.rooms if r.unit == unit_id and not (r.void or r.outdoor or r.stair)]
        if not rooms:
            continue
        measured = [r for r in rooms if not r.broken]
        largest = max(measured, key=lambda r: r.net, default=None)
        if largest is not None and largest.net >= OTURMA_ODASI - PAY:
            continue
        broken = [r for r in rooms if r.broken]
        if broken:
            findings.append(
                ext.AuditFinding(
                    message=(
                        f"{bolum(unit)}: {mahal(broken[0])} mahalinin sınırı bulunamadı, alanı güvenilir değil; "
                        f"net alanı en az {metrekare(OTURMA_ODASI)} olan bir oturma odası denetlenemedi — {MADDE}."
                    ),
                    severity="info",
                    feature_id=broken[0].uid,
                )
            )
            continue
        if largest is None:
            continue
        findings.append(
            ext.AuditFinding(
                message=(
                    f"{bolum(unit)}: net alanı en az {metrekare(OTURMA_ODASI)} olan piyes yok (en büyüğü "
                    f"{mahal(largest)}, {metrekare(largest.net)}); her konutta net alanı en az "
                    f"{metrekare(OTURMA_ODASI)}, dar kenarı en az 3.00 m bir oturma odası bulunur — {MADDE}."
                ),
                severity="error",
                feature_id=largest.uid,
            )
        )
    return findings
