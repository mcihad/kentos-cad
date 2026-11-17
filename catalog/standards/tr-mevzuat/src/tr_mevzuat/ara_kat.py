"""Ara katlı daire (BYKHY 48/2'nin ikinci ve üçüncü cümleleri).

Madde 48/2 (değişik 2009/15316 BKK): “İkiden çok ara kat bulunmayan apartman dairelerinde tek kapı bulunması hâlinde,
bu kapı üst katta düzenlenemez. Üstteki katın döşeme alanı, bu kat için ayrı bir çıkış sağlanmadıkça 70 m2’yi
aşamaz.”

Olgularla karar verilen: iki katlı konut bölümü (``UnitFacts.levels``'ta iki kat). "Ara kat" yönetmelikte tanımlı
değil (43/1 onu kattan ayrı sayar); iki katlı daire her okuyuşta bu cümlelerin içindedir, üst katı da yüksek olanıdır.
Üç ve daha çok katlı dairede hangisinin "üstteki kat" olduğu ve kaç ara kat sayıldığı belli değil: karar verilmez.
Kapılar olguların konut kapılarıdır (``doors``): üst katta kapı varsa o katın ayrı çıkışıdır. "Döşeme alanı"nın net mi
brüt mü olduğu yazılı değil: net alan 70 m²'yi aşıyorsa aykırıdır, brüt aşıp net aşmıyorsa karar verilemez. Fıkra
"Birinci fıkrada belirtilenler dışındaki konutlarda" diye başladığı için 48/1'in muaf tuttuğu binalarda okunmaz.
"""

from __future__ import annotations

from kentos import ext

from .bina import PAY, bina, bolum, kat_adlari, konutlar, metrekare

#: BYKHY 48/2: ayrı çıkışı olmayan üst katın en çok döşeme alanı, m².
UST_KAT = 70.0
MADDE = "BYKHY md. 48/2 (2009/15316 sayılı BKK ile değişik)"


@ext.audit("ara-katli-daire")
def ara_katli_daire(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """İki katlı konut bölümünde tek kapı üst katta olamaz (hata); üst katta kapısı yoksa üst katın döşeme alanı 70
    m²'yi aşamaz (net alanı aşıyorsa hata; yalnız brütü aşıyorsa bilgi, karar verilemedi). Bodrum dahil 4 katı
    geçmeyen konut binasında ve tek evde (48/1) okunmaz."""
    a = facts.project.architecture
    if bina(a).kacis_mesafesi_aranmaz:
        return []
    order = {s.id: i for i, s in enumerate(a.levels)}
    names = kat_adlari(a)
    findings: list[ext.AuditFinding] = []
    for unit_id, unit in konutlar(a).items():
        storeys = [x for x in unit.levels if x.level in order]
        if len(storeys) != 2:
            continue
        upper = max(storeys, key=lambda x: order[str(x.level)])
        doors = [d for d in a.doors if d.unit == unit_id]
        own = [d for d in doors if d.level == upper.level]
        where = f"{bolum(unit)}, üst katı {names.get(upper.level, upper.level)}"
        if len(doors) == 1 and own:
            findings.append(
                ext.AuditFinding(
                    message=(
                        f"{where}: dairenin tek kapısı üst katta; ikiden çok ara kat bulunmayan apartman dairelerinde "
                        f"tek kapı bulunması hâlinde bu kapı üst katta düzenlenemez — {MADDE}."
                    ),
                    severity="error",
                    feature_id=own[0].uid,
                )
            )
            continue
        if own or upper.gross <= UST_KAT + PAY:
            continue
        rule = (
            f"üstteki katın döşeme alanı, bu kat için ayrı bir çıkış sağlanmadıkça {metrekare(UST_KAT)}'yi aşamaz"
        )
        if upper.net > UST_KAT + PAY:
            text = f"üst katta kapı yok ve net alanı {metrekare(upper.net)} (brüt {metrekare(upper.gross)}); {rule}"
            severity = "error"
        else:
            text = (
                f"üst katta kapı yok; net alanı {metrekare(upper.net)}, brüt alanı {metrekare(upper.gross)}; {rule}; "
                f"yönetmelik döşeme alanının net mi brüt mü olduğunu söylemiyor, karar verilemedi"
            )
            severity = "info"
        findings.append(ext.AuditFinding(message=f"{where}: {text} — {MADDE}.", severity=severity))
    return findings
