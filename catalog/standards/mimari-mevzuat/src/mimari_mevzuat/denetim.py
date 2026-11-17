"""Mevzuat kurallarının KentOS eklenti denetimine katılan yüzü (manifestteki ``audits``; docs/adr/8078): bulut işi ve
Eklenti denetimi aynı kuralı ``kentos.ext`` üzerinden çalıştırır. Kaldı hatadır; eksik veri uyarı, uzman incelemesi
bilgidir: ikisi de iletinin başında söylenir.
"""

from kentos import ext

from . import mimari

_ONEK = {"missingData": "Eksik veri: ", "expertReview": "Uzman incelemesi: "}
_AGIRLIK = {"missingData": "warning", "expertReview": "info"}


def _bulgular(facts: ext.GisAuditFacts, kodlar: tuple[str, ...]) -> list[ext.AuditFinding]:
    """Kuralların bulgularından kodu ``kodlar``'la başlayanlar (ve kural setinin kendi bulguları)."""
    project = facts.project
    cevap = mimari.denetle({"building": project.building, "permit": project.permit.to_json()})
    out = []
    for b in cevap["findings"]:
        if not b["code"].startswith(kodlar + ("regulation_",)):
            continue
        durum = b.get("state")
        kural = f" [{b['rule']}]" if b.get("rule") else ""
        out.append(
            ext.AuditFinding(
                message=f"{_ONEK.get(durum, '')}{b['message']}{kural}",
                severity=_AGIRLIK.get(durum, "error"),
            )
        )
    return out


@ext.audit("asansor")
def asansor_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Asansör (PAİY 34/1 ve 34/8; Sivas 34/1 ve 34/8): başvuru gününün kural setiyle."""
    return _bulgular(facts, ("paiy_elevator_", "regulation_conflict"))


@ext.audit("kapilar")
def kapi_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Kaçış yolu kapıları (BYKHY 47/2 açılış yönü, 47/3 kaçış merdiveni ve yangın güvenlik holü kapıları)."""
    return _bulgular(facts, ("bykhy_exit_door", "bykhy_stair_door"))


@ext.audit("emsal")
def emsal_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Emsalin 22. madde sınırlarının koşulları (22/1-ç kapıcı dairesinin 44/1'e göre zorunluluğu, 22/1-f kütüphane)."""
    return _bulgular(facts, ("paiy_caretaker_", "paiy_library_"))


@ext.audit("merdiven-alti")
def merdiven_alti_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Merdiven altı (PAİY 31/6): 1,80 m'den alçak merdiven altı kapatılır."""
    return _bulgular(facts, ("paiy_under_stair_",))


@ext.audit("kompartiman")
def kompartiman_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Yangın kompartımanları (BYKHY 24/2 tehlikeli alanlar, 24/6 ve Ek-4 en çok alan, 24/7 açıklıklar)."""
    return _bulgular(facts, ("bykhy_compartment_", "bykhy_hazard_room"))
