"""Mevzuat kurallarının KentOS eklenti denetimine katılan yüzü (manifestteki ``audits``; docs/adr/8078): bulut işi ve
Eklenti denetimi aynı kuralı ``kentos.ext`` üzerinden çalıştırır. Kaldı hatadır; eksik veri uyarı, uzman incelemesi
bilgidir: ikisi de iletinin başında söylenir.
"""

from kentos import ext

from . import mimari

_ONEK = {"missingData": "Eksik veri: ", "expertReview": "Uzman incelemesi: "}
_AGIRLIK = {"missingData": "warning", "expertReview": "info"}


@ext.audit("asansor")
def asansor_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Asansör (PAİY 34/1 ve 34/8; Sivas 34/1 ve 34/8): başvuru gününün kural setiyle."""
    project = facts.project
    cevap = mimari.denetle({"building": project.building, "permit": project.permit.to_json()})
    out = []
    for b in cevap["findings"]:
        durum = b.get("state")
        kural = f" [{b['rule']}]" if b.get("rule") else ""
        out.append(
            ext.AuditFinding(
                message=f"{_ONEK.get(durum, '')}{b['message']}{kural}",
                severity=_AGIRLIK.get(durum, "error"),
            )
        )
    return out
