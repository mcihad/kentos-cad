"""Binaların Yangından Korunması Hakkında Yönetmelik (BYKHY, RG 19.12.2007/26735) kuralları.

Olgular kaçış yürüyüşlerini yönetmeliğin ölçtüğü gibi verir: mahalin en uzak noktası duvarlardan 40 cm önde (32/6),
en yakın kat çıkışına yürüyüş (4/u, 32/4), başka çıkışa giden yolla ortak kalan tek yön payı (4/kk) ve ulaşılan
çıkış sayısı; konutun mahalinden konut kapısına (48/2) ve konut kapısından en yakın kat çıkışına (48/4) yürüyüş.

- Madde 48/1 (değişik 2009/15316 BKK): “Bodrum katlar dâhil 4 katı geçmeyen konutlar ile tek evler, ikiz evler ve
  sıra evler gibi konutlarda … kaçışlar, kaçış mesafesi aranmaksızın normal merdivenlerle sağlanabilir.”
- Madde 48/2: “Birinci fıkrada belirtilenler dışındaki konutlarda, konut içindeki herhangi bir noktadan konut çıkış
  kapısına kadar olan uzaklığın 20 m’yi, yağmurlama sistemi olan konutlarda 30 m’yi geçmemesi gerekir.”
- Madde 48/4: “Kaçış uzaklığı, apartman dairelerinin kapısından başlanarak ölçülür.” Madde 32/3: “Kaçış uzaklığı,
  kullanım sınıfına göre Ek-5/B’de belirtilen değerlerden daha büyük olamaz.” Ek-5/B (değişik 2009/15316 ve
  2015/7401 BKK), Apartmanlar: tek yön en çok 15 m (yağmurlama sistemi yok) ve 30 m (yağmurlama sistemli); iki yön
  30 m ve 75 m.
- Madde 31/3: “Her katta, o katın kullanıcı yüküne ve en uzun kaçış uzaklığına göre çıkış imkânları sağlanır.”
- Madde 33/6-b: “Bir yapıda veya katlarında bulunan her kullanıcı için, diğer kullanıcıların kullanımında olan
  odalardan veya mekânlardan geçmek zorunda kalınmaksızın, bir çıkışa veya çıkışlara doğrudan erişim sağlanması
  gerekir.”

Olgular binada yağmurlama sistemi olup olmadığını söylemez: yalnız yağmurlama sistemi olmayan binanın sınırını aşan
ölçü uyarıdır, iki sınırı da aşan hatadır. Konut olmayan bölümün Ek-5/B'deki kullanım sınıfı da olgularda yok
(niteliği tapudaki serbest yazıdır): o bölümlerin kaçış uzaklığına bu sürümde karar verilmez.
"""

from __future__ import annotations

from kentos import ext

from .bina import PAY, bina, bolum, kat_adlari, konutlar, mahal, metre

#: BYKHY 48/2: konutun içinde konut çıkış kapısına en çok uzaklık, m: yağmurlama sistemi yok, var.
DAIRE_ICI = (20.0, 30.0)
#: BYKHY Ek-5/B, Apartmanlar: tek yönde ve iki yönde en çok kaçış uzaklığı, m: yağmurlama sistemi yok, var.
TEK_YON = (15.0, 30.0)
IKI_YON = (30.0, 75.0)

MADDE_DAIRE_ICI = "BYKHY md. 48/2 (2009/15316 sayılı BKK ile değişik)"
MADDE_DAIRE_KAPISI = "BYKHY md. 32/3, 48/4 ve Ek-5/B (apartmanlar)"
MADDE_KAT_CIKISI = "BYKHY md. 31/3"
MADDE_ERISIM = "BYKHY md. 33/6-b"


@ext.audit("daire-ici")
def daire_ici(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Konutun her katında, mahallerin en uzak noktasından konut kapısına en uzun yürüyüş 20 m'yi aşarsa uyarı
    (yağmurlama sistemi yoksa sınır aşılıyor), 30 m'yi de aşarsa hata. Konutun o katta kapısı olduğu hâlde kapıya yol
    bulunmayan mahal hata. Bodrum dahil 4 katı geçmeyen konut binasında ve tek evde (48/1) bir şey söylenmez."""
    a = facts.project.architecture
    if bina(a).kacis_mesafesi_aranmaz:
        return []
    dwellings = konutlar(a)
    levels = kat_adlari(a)
    findings: list[ext.AuditFinding] = []
    farthest: dict[tuple[str, str | None], tuple[float, ext.RoomFacts]] = {}
    for room in a.rooms:
        if room.unit not in dwellings or room.escape is None:
            continue
        entrance = room.escape.entrance
        if entrance is None:
            if room.escape.has_entrance:
                findings.append(
                    ext.AuditFinding(
                        message=(
                            f"{mahal(room)} ({levels.get(room.level, room.level)}): konutun kapısına mahalden "
                            f"yürüyüş yolu yok; konut içindeki her noktadan konut çıkış kapısına uzaklık "
                            f"{metre(DAIRE_ICI[0])}'yi geçemez — {MADDE_DAIRE_ICI}."
                        ),
                        severity="error",
                        feature_id=room.uid,
                    )
                )
            continue
        key = (str(room.unit), room.level)
        if key not in farthest or entrance.length > farthest[key][0]:
            farthest[key] = (entrance.length, room)
    for (unit_id, level), (length, room) in farthest.items():
        if length <= DAIRE_ICI[0] + PAY:
            continue
        wet = length > DAIRE_ICI[1] + PAY
        where = f"{bolum(dwellings[unit_id])}, {levels.get(level, level)}"
        rule = (
            f"konut içindeki herhangi bir noktadan konut çıkış kapısına uzaklık {metre(DAIRE_ICI[0])}'yi, yağmurlama "
            f"sistemi olan konutlarda {metre(DAIRE_ICI[1])}'yi geçemez"
        )
        verdict = (
            "yağmurlama sistemi olsa da sınır aşılıyor"
            if wet
            else "binada yağmurlama sistemi yoksa sınır aşılıyor"
        )
        findings.append(
            ext.AuditFinding(
                message=(
                    f"{where}: {mahal(room)} mahalinin en uzak noktasından konutun kapısına {metre(length)}; {rule}; "
                    f"{verdict} — {MADDE_DAIRE_ICI}."
                ),
                severity="error" if wet else "warning",
                feature_id=room.uid,
            )
        )
    return findings


def _olcu(walk: ext.EscapeWalk, column: int) -> tuple[str, float, float]:
    """Ek-5/B'nin bir sütununa göre yürüyüşün ölçülen yanı, ölçüsü ve sınırı: tek çıkışta bütün yol tek yöndür; iki
    çıkışta önce yolun tek yönlü kısmı tek yön sınırına, o aşılmıyorsa bütün yol iki yön sınırına."""
    one, two = TEK_YON[column], IKI_YON[column]
    if (walk.exits or 0) <= 1:
        return ("tek yönde", walk.length, one)
    one_way = walk.one_way or 0.0
    if one_way > one + PAY:
        return ("yolun tek yönlü kısmı", one_way, one)
    return ("iki yönde", walk.length, two)


@ext.audit("daire-kapisi")
def daire_kapisi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Her konut kapısından en yakın kat çıkışına yürüyüş Ek-5/B'nin apartmanlar satırına göre: yalnız yağmurlama
    sistemi olmayan binanın sınırını aşarsa uyarı, yağmurlama sistemli binanınkini de aşarsa hata. Kaçış merdiveni
    mahaline açılan ya da hiçbir çıkışa ulaşmayan kapının yürüyüşü olgularda yok; ona karar verilmez. Bodrum dahil 4
    katı geçmeyen konut binasında ve tek evde (48/1) bir şey söylenmez."""
    a = facts.project.architecture
    if bina(a).kacis_mesafesi_aranmaz:
        return []
    dwellings = konutlar(a)
    levels = kat_adlari(a)
    findings: list[ext.AuditFinding] = []
    for door in a.doors:
        walk = door.exit
        unit = dwellings.get(door.unit)
        if walk is None or unit is None:
            continue
        dry = _olcu(walk, 0)
        wet = _olcu(walk, 1)
        if wet[1] > wet[2] + PAY:
            (words, measure, limit), severity = wet, "error"
            verdict = f"yağmurlama sistemi olsa da sınır ({metre(limit)}) aşılıyor"
        elif dry[1] > dry[2] + PAY:
            (words, measure, limit), severity = dry, "warning"
            verdict = f"binada yağmurlama sistemi yoksa sınır ({metre(limit)}) aşılıyor"
        else:
            continue
        findings.append(
            ext.AuditFinding(
                message=(
                    f"{bolum(unit)}, {levels.get(door.level, door.level)}: kapısından en yakın kat çıkışına "
                    f"{words} {metre(measure)}; apartmanlarda kaçış uzaklığı kapıdan ölçülür, tek yönde en çok "
                    f"{metre(TEK_YON[0])} (yağmurlama sistemiyle {metre(TEK_YON[1])}), iki yönde "
                    f"{metre(IKI_YON[0])} ({metre(IKI_YON[1])}); {verdict} — {MADDE_DAIRE_KAPISI}."
                ),
                severity=severity,
                feature_id=door.uid,
            )
        )
    return findings


@ext.audit("cikis-erisimi")
def cikis_erisimi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Mahali olup kat çıkışı olmayan kat hata (31/3: kaçış merdiveni mahali, açık merdiven ya da ±0.00 ve altında
    dışarı açılan kapı). Çıkışı olan katta kendi bölümünün mahallerinden ve ortak alanlardan geçerek hiçbir kat
    çıkışına ulaşmayan mahal hata (33/6-b). Yürüyüşü olmayan mahal (boşluk, dış mahal, sınırı bulunamayan, kaçış
    merdiveni) denetlenmez."""
    a = facts.project.architecture
    levels = kat_adlari(a)
    findings: list[ext.AuditFinding] = []
    without: set[str | None] = set()
    for room in a.rooms:
        if room.escape is None:
            continue
        level = levels.get(room.level, room.level)
        if a.exits(room.level) == 0:
            if room.level not in without:
                without.add(room.level)
                findings.append(
                    ext.AuditFinding(
                        message=(
                            f"{level}: katta kat çıkışı yok (kaçış merdiveni mahali, açık merdiven ya da ±0.00 ve "
                            f"altındaki katta dışarı açılan kapı); her katta çıkış imkânları sağlanır — "
                            f"{MADDE_KAT_CIKISI}."
                        ),
                        severity="error",
                    )
                )
            continue
        if room.escape.exit is None:
            findings.append(
                ext.AuditFinding(
                    message=(
                        f"{mahal(room)} ({level}): kendi bölümünün mahallerinden ve ortak alanlardan geçerek hiçbir "
                        f"kat çıkışına ulaşılmıyor; her kullanıcı başka kullanıcıların odalarından geçmeden bir "
                        f"çıkışa doğrudan erişebilmeli — {MADDE_ERISIM}."
                    ),
                    severity="error",
                    feature_id=room.uid,
                )
            )
    return findings
