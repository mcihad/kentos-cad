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
    """Emsalin 22. madde sınırlarının koşulları (22/1-ç kapıcı dairesinin 44/1'e göre zorunluluğu, 22/1-f kütüphane,
    22/1-m geleneksel kesme taş yığma binanın taşlık, dış sofa ve eyvanı, 20/8-j ve 5/8 kalın taş duvarları, Sivas 22/k)."""
    return _bulgular(facts, ("paiy_caretaker_", "paiy_library_", "paiy_traditional_", "paiy_masonry_", "local_masonry_"))


@ext.audit("merdiven-alti")
def merdiven_alti_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Merdiven altı (PAİY 31/6): 1,80 m'den alçak merdiven altı kapatılır."""
    return _bulgular(facts, ("paiy_under_stair_",))


@ext.audit("kompartiman")
def kompartiman_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Yangın kompartımanları (BYKHY 24/2 tehlikeli alanlar, 24/6 ve Ek-4 en çok alan, 24/7 açıklıklar)."""
    return _bulgular(facts, ("bykhy_compartment_", "bykhy_hazard_room"))


@ext.audit("dayanim")
def dayanim_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Yapı elemanlarının yangına dayanım süreleri (BYKHY Ek-3/B, Ek-3/C): beyan edilen sınıf istenenle."""
    return _bulgular(facts, ("bykhy_element_resistance", "bykhy_height_not_permitted"))


@ext.audit("kaplamalar")
def kaplama_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Kaplamaların yanıcılığı (BYKHY 26/2 döşeme kaplaması, 26/4 tavan kaplaması ve asma tavan; Ek-2/Ç)."""
    return _bulgular(facts, ("bykhy_floor_covering_reaction", "bykhy_ceiling_reaction"))


@ext.audit("catilar")
def cati_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Çatıların yanıcılığı (BYKHY 28/2 kaplamanın BROOF sınıfı ve altındaki katmanlar, 28/3-b yüksek ve bitişik nizam
    binada yanmaz çatı; Ek-2/Ç)."""
    return _bulgular(facts, ("bykhy_roof_",))


@ext.audit("kuyular")
def kuyu_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Şaftlar ve asansörler (BYKHY 25/3 yüksek binada şaft ve baca, 41/9 kaçış merdivenine açılan şaft kapağı, 62/2
    asansör kuyusu, 62/4 ve 62/6 asansör kapısının önü, 62/7 ve Ek-3/B 12 kat kapısının dayanımı)."""
    return _bulgular(facts, ("bykhy_shaft_", "bykhy_elevator_"))


@ext.audit("kat-yuksekligi")
def kat_yuksekligi_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Kat yüksekliği (PAİY 28/1-c; Sivas 28/1-c): konut bölgesinde zemin ve normal katlar, başvuru gününün hâliyle."""
    return _bulgular(facts, ("paiy_storey_height_",))


@ext.audit("bina-yuksekligi")
def bina_yuksekligi_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Bina yüksekliği ve kat adedi (PAİY 5/11): parselin plan notunun değerleri."""
    return _bulgular(facts, ("paiy_building_height_", "paiy_storey_count_"))


@ext.audit("kapi-olculeri")
def kapi_olculeri_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Kapı yüksekliği ve net genişliği (PAİY 39/1; Sivas 39/1): bina ve bölüm girişi, öbür kapılar."""
    return _bulgular(facts, ("paiy_door_", "paiy_revolving_door_"))


@ext.audit("kullanim")
def kullanim_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Kullanım sınıfına özel kaçış hükümleri (BYKHY 49/1-a hasta odası, 50/1-a, 50/1-b ve 50/3 otel, 52 fabrika,
    mağaza, depo, büro ve ayakta tedavi merkezinin kaçış merdivenleri)."""
    return _bulgular(facts, ("bykhy_patient_room_", "bykhy_hotel_", "bykhy_single_stair"))


@ext.audit("saglik")
def saglik_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Sağlık yapılarının yatılan katları ve hastane koridorları (BYKHY 49/1-b kompartımanlar, 49/2 koridor
    genişliği)."""
    return _bulgular(facts, ("bykhy_patient_floor_", "bykhy_hospital_"))


@ext.audit("kazan")
def kazan_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Kazan dairesi ve yakıt deposu (BYKHY 54/2 ayrım, 54/4 kapının açıldığı yer, 54/5 çıkışlar ve kapılar, 56/1 yakıt
    deposu)."""
    return _bulgular(facts, ("bykhy_boiler_room_", "bykhy_fuel_store_"))


@ext.audit("yagmurlama")
def yagmurlama_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Otomatik yağmurlama sisteminin zorunlu olduğu binalar (BYKHY 96/2)."""
    return _bulgular(facts, ("bykhy_sprinklers_",))

