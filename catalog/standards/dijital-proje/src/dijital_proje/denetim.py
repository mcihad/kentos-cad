"""Dijital proje yönetmeliğinin KentOS denetimine katılan kuralları (manifestteki ``audits``; docs/adr/8063).

Kural çizimi okumaz: KentOS'un verdiği olgulara (``facts.project.permit``) bakar ve kararı ``kapsam``'a bırakır.
Teyit gereken satır ve okunamayan girdi uyarıdır; öbür satırlar bilgi olarak yazılır, çünkü bir yükümlülüğün
başlaması çizimde düzeltilecek bir kusur değildir.
"""

from kentos import ext

from . import kapsam


@ext.audit("kapsam")
def kapsam_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Dijital teslim kapsamı: her yükümlülüğün bu projedeki durumu, maddesiyle."""
    permit = facts.project.permit
    if not permit.particulars:
        return [
            ext.AuditFinding(
                message="Dijital teslim kapsamı için künyede bilgi yok: Ruhsat takımı penceresinin “Dijital teslim "
                "kapsamı” bölümünde başvuru tarihini, kullanımı ve nüfusları yazın.",
                severity="info",
            )
        ]
    sonuc = kapsam.karar(permit.to_json())
    bulgular = [ext.AuditFinding(message=s["message"], severity="warning") for s in sonuc["problems"]]
    for satir in sonuc["rows"]:
        gun = f" ({_tarih(satir['from'])}'den)" if satir["from"] else ""
        maddeler = ", ".join(satir["articles"])
        bulgular.append(
            ext.AuditFinding(
                message=f"{satir['title']}: {satir['words']}{gun}. {satir['reason']} [{maddeler}]",
                severity="warning" if satir["state"] == "confirm" else "info",
            )
        )
    return bulgular


def _tarih(iso: str) -> str:
    yil, ay, gun = iso.split("-")
    return f"{int(gun)}.{int(ay)}.{yil}"
