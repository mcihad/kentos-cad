"""Dijital proje yönetmeliğinin KentOS denetimine katılan kuralları (manifestteki ``audits``; docs/adr/8063).

Kural çizimi okumaz: KentOS'un verdiği olgulara (``facts.project.permit``) bakar ve kararı ``kapsam``'a bırakır.
Teyit gereken satır ve okunamayan girdi uyarıdır; öbür satırlar bilgi olarak yazılır, çünkü bir yükümlülüğün
başlaması çizimde düzeltilecek bir kusur değildir.
"""

from kentos import ext

from . import ek34, kapsam


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


@ext.audit("ek34")
def ek34_denetimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """EK-3 ve EK-4: mahal numarası ve adı, katman, tarama, aks çizgisi, ölçü yazısı ve blok adı (Md. 7)."""
    bulgular = []
    for b in ek34.denetle({"cad": facts.project.cad}):
        fix = None
        if b["fix"]:
            fix = ext.SemanticFix(title=b["fix"]["label"], command=b["fix"]["command"], args=b["fix"]["input"])
        nesne = b["uids"][0] if b["uids"] else None
        bulgular.append(ext.AuditFinding(message=f"{b['message']} [{b['cite']}]", severity=b["severity"],
                                         feature_id=nesne, fix=fix))
    return bulgular
