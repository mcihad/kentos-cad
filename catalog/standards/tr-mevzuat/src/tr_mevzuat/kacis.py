"""Kaçış uzaklığı, konut olmayan bölümlerde (BYKHY md. 32/3-4 ve Ek-5/B).

Madde 32/3: “Kaçış uzaklığı, kullanım sınıfına göre Ek-5/B’de belirtilen değerlerden daha büyük olamaz.” Ek-5/B'nin
(Çıkışlara Götüren En Uzun Kaçış Uzaklıkları ve Birim Genişlikleri; 2009/15316 ve 2015/7401 sayılı BKK ile değişik)
kaçış uzaklığı sütunları, m: tek yön ve iki yön, yağmurlama sistemi yok ve var. Dipnot (1): “Kolay alevlenici malzeme
üretimi yapmayan endüstriyel amaçlı yapılarda tek ve iki yönlü uzaklık ½ oranında artırılabilir.”

Ölçü KentOS Mimari'nin yangın denetiminin okuduğudur: mahalin en uzak noktasından en yakın kat çıkışına yürüyüş; tek
çıkışta bütün yol tek yöndür, iki çıkışta önce yolun tek yönlü kısmı tek yön sınırına, o aşılmıyorsa bütün yol iki yön
sınırına. Bölümün sınıfı olguların ``occupancy``'sidir (niteliğinden okunur: Büro, Dükkân, Otel …); niteliği bir sınıf
söylemeyen bölüm binanın bilinen en ağır sınıfıyla, o da yoksa yerel denetim gibi büro sayılır ve bulgu bunu söyler.
Bir bölüme ait olmayan mahal (ortak alanlar, bodrumdaki otopark ya da depo) yerel denetim gibi önce kendi adının
söylediği sınıfla (mahalin ``occupancy``'si: Otopark, Depo, Lokanta …), o da yoksa binanınkiyle ölçülür. Konutun
mahalleri 48'in kurallarınındır; yalnız konut olan binanın, adı bir sınıf söylemeyen ortak mahalleri ölçülmez (48/4
kapıdan ölçer).
"""

from __future__ import annotations

from kentos import ext

from .bina import PAY, Yagmurlama, kat_adlari, mahal, metre, yagmurlama

#: Ek-5/B: sınıfın satırı, tek yön ve iki yön (yağmurlama sistemi yok, var), m.
UZAKLIK: dict[str, tuple[str, tuple[float, float], tuple[float, float]]] = {
    "highHazard": ("yüksek tehlikeli yerler", (10.0, 20.0), (20.0, 35.0)),
    "industry": ("endüstri amaçlı yapılar", (15.0, 25.0), (30.0, 60.0)),
    "dormitory": ("yurtlar, yatakhaneler", (15.0, 30.0), (45.0, 75.0)),
    "shop": ("mağazalar, dükkânlar, marketler", (15.0, 25.0), (45.0, 60.0)),
    "office": ("büro binaları", (15.0, 30.0), (45.0, 75.0)),
    "parking": ("otoparklar ve depolar", (15.0, 25.0), (45.0, 60.0)),
    "school": ("okul ve eğitim yapıları", (15.0, 30.0), (45.0, 75.0)),
    "assembly": ("toplanma amaçlı binalar", (15.0, 25.0), (45.0, 60.0)),
    "hospital": ("hastaneler, huzurevleri", (15.0, 25.0), (30.0, 45.0)),
    "hotel": ("oteller, pansiyonlar", (15.0, 20.0), (30.0, 45.0)),
}
MADDE = "BYKHY md. 32/3-4 ve Ek-5/B (2009/15316 ve 2015/7401 sayılı BKK ile değişik)"


def _agir(sinif: str) -> tuple[float, float]:
    """İki sınıftan ağırı kısa yürüyüşlüsüdür: önce iki yön, sonra tek yön sınırı (yağmurlamasız)."""
    _, tek, iki = UZAKLIK[sinif]
    return (iki[0], tek[0])


def _olcu(walk: ext.EscapeWalk, tek: tuple[float, float], iki: tuple[float, float], sutun: int):
    """Bir sütuna göre yürüyüşün ölçülen yanı, ölçüsü ve sınırı: tek çıkışta bütün yol tek yön; iki çıkışta önce yolun
    tek yönlü kısmı, o aşılmıyorsa bütün yol iki yön."""
    if (walk.exits or 0) <= 1:
        return "tek yönde", walk.length, tek[sutun]
    one_way = walk.one_way or 0.0
    if one_way > tek[sutun] + PAY:
        return "yolun tek yönlü kısmı", one_way, tek[sutun]
    return "iki yönde", walk.length, iki[sutun]


def _karar(walk: ext.EscapeWalk, sinif: str, wet: Yagmurlama) -> tuple[str, str, float, str] | None:
    """Önemi, ölçülen yanı, ölçüsü ve sözü; aşmıyorsa yok. Ayar biliniyorsa onun sütunu (aşan hata); bilinmiyorsa
    yağmurlamalı sütunu da aşan hata, yalnız yağmurlamasızı aşan uyarı. Endüstri yapısında dipnot (1)'in ½ artışıyla
    uygun olan uyarıdır: malzemenin kolay alevlenici olup olmadığı olgularda yok."""
    _, tek, iki = UZAKLIK[sinif]
    sutunlar = [wet.sutun] if wet.sutun is not None else [1, 0]
    for sutun in sutunlar:
        words, measure, limit = _olcu(walk, tek, iki, sutun)
        if measure <= limit + PAY:
            continue
        if wet.sutun is None:
            severity = "error" if sutun == 1 else "warning"
            verdict = (
                f"{wet.soz}; yağmurlama sistemi olsa da sınır ({metre(limit)}) aşılıyor"
                if sutun == 1
                else f"{wet.soz}; binada yağmurlama sistemi yoksa sınır ({metre(limit)}) aşılıyor"
            )
        else:
            severity, verdict = "error", f"{wet.soz}, sınır {metre(limit)}"
        if sinif == "industry" and measure <= limit * 1.5 + PAY:
            severity = "warning"
            verdict += (
                f"; kolay alevlenici malzeme üretilmiyorsa dipnot (1) ile sınır {metre(limit * 1.5)}, karar "
                f"verilemedi"
            )
        return severity, words, measure, verdict
    return None


@ext.audit("kacis-uzakligi")
def kacis_uzakligi(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Konut olmayan bölümlerin (ve onların bulunduğu binanın ortak mahallerinin) her mahalinden en yakın kat çıkışına
    yürüyüş, bölümün Ek-5/B satırının sınırına, projenin yağmurlama ayarının sütunuyla: aşarsa hata. Sınıfı okunamayan
    bölüm binanın bilinen en ağır sınıfıyla ya da büro sayılır, bulgu söyler. Bölümü olmayan mahal önce kendi adının
    sınıfıyla ölçülür (konut binasının bodrum otoparkı da). Konutun mahalleri ve yalnız konut olan binanın adı bir sınıf
    söylemeyen ortak mahalleri okunmaz; yürüyüşü olmayan mahal (boşluk, dış mahal, sınırı bulunamayan, kaçış merdiveni)
    ve hiçbir çıkışa ulaşmayan mahal (çıkış erişimi kuralının) denetlenmez."""
    a = facts.project.architecture
    units = {u.id: u for u in a.units}
    bilinen = [
        u.occupancy for u in a.units if u.kind == "unit" and not u.dwelling and u.occupancy in UZAKLIK
    ]
    bina_sinifi = min(bilinen, key=_agir) if bilinen else None
    konut_disi = any(u.kind == "unit" and not u.dwelling for u in a.units)
    levels = kat_adlari(a)
    wet = yagmurlama(a)
    findings: list[ext.AuditFinding] = []
    for room in a.rooms:
        if room.escape is None or room.escape.exit is None:
            continue
        unit = units.get(room.unit) if room.unit is not None else None
        if unit is not None and unit.kind == "annex" and unit.annex_of is not None:
            unit = units.get(unit.annex_of, unit)
        if unit is not None and unit.kind == "unit":
            if unit.dwelling:
                continue
            sinif, okunan = (unit.occupancy, True) if unit.occupancy in UZAKLIK else (bina_sinifi, False)
            adindan = False
        elif room.occupancy in UZAKLIK:
            # Bölümü olmayan mahal kendi adının sınıfıyla, yerel denetim gibi: Otopark, Depo, Lokanta …
            sinif, okunan, adindan = room.occupancy, True, True
        elif konut_disi:
            sinif, okunan, adindan = bina_sinifi, bina_sinifi is not None, False
        else:
            continue
        not_ = "; sınıf mahalin adından okundu" if adindan else ""
        if sinif is None:
            sinif, not_ = "office", "; bölümün niteliği bir kullanım sınıfı söylemiyor, büro sayıldı"
        elif not okunan:
            not_ = f"; bölümün niteliği bir kullanım sınıfı söylemiyor, binanın {UZAKLIK[sinif][0]} sınıfı okundu"
        decided = _karar(room.escape.exit, sinif, wet)
        if decided is None:
            continue
        severity, words, measure, verdict = decided
        satir, tek, iki = UZAKLIK[sinif]
        findings.append(
            ext.AuditFinding(
                message=(
                    f"{mahal(room)} ({levels.get(room.level, room.level)}): en uzak noktasından en yakın kat çıkışına "
                    f"{words} {metre(measure)}; Ek-5/B'de {satir} için tek yönde en çok {metre(tek[0])} (yağmurlama "
                    f"sistemiyle {metre(tek[1])}), iki yönde {metre(iki[0])} ({metre(iki[1])}){not_}; {verdict} — "
                    f"{MADDE}."
                ),
                severity=severity,
                feature_id=room.uid,
            )
        )
    return findings
