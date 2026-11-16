"""Çıkmaz koridor (BYKHY 4/i ve Ek-5/B).

- Madde 4/i (değişik 2009/15316 BKK): “Çıkmaz koridor mesafesi: Mekân içerisinden mekânın koridora bağlanan kapısına
  kadar olan mesafe göz önüne alınmaksızın, kaçışta, mekânların bağlı olduğu koridorun en uzak noktasından koridor
  boyunca bir çıkışa veya iki yönde kaçış imkânına sahip olunan noktaya kadar olan mesafeyi,”
- Ek-5/B'nin "Çıkmaz koridor en çok uzaklık (m)" sütunu: yağmurlama sistemi yoksa 15 m, yağmurlama sistemiyle 20 m;
  yalnız yüksek tehlikeli yerlerde yağmurlama sistemi yoksa 10 m. Apartmanlar satırı da 15 ve 20 m'dir.

Ölçü KentOS Mimari'nin yangın denetiminin okuduğudur: türü ``corridor`` olan mahalin kendi yürüyüşünün tek yön payı
(en uzak noktasından bir kat çıkışına ya da iki yönün ayrıldığı noktaya; tek çıkışta bütün yol). Sütun yüksek
tehlikeli yerler dışında her sınıfta aynıdır: konut olmayan bölümlerin ya da karma binanın koridorunda 15/20 m'yi aşan
çıkmaz her sınıf için aykırıdır. Koridorun sınıfı kaçış uzaklığı kuralındaki gibi okunur (bölümünün sınıfı; bölümü
yoksa kendi adının, o da yoksa binanın en ağır sınıfı, 31/5 ve 18): yüksek tehlikeli yerse (md. 8/1-ğ ve 17: LPG,
akaryakıt, parlayıcı ve patlayıcı madde, yanıcı sıvı; olgularda bölümün niteliğinden ya da mahalin adından) yağmurlama
sistemi yoksa sınır 10 m. Yalnız konut bölümleri olan binanın ortak koridoru apartmanlar satırıyla okunur ve 48/1'in
muafiyeti ona da uygulanır (çıkmaz koridor mesafesi bir kaçış mesafesidir). Konutun kendi holü 48/2'nindir, okunmaz.
"""

from __future__ import annotations

from kentos import ext

from .bina import bina, karar, kat_adlari, mahal, metre, yagmurlama
from .kacis import UZAKLIK, agir

#: Ek-5/B, çıkmaz koridor: yağmurlama sistemi yok, var (m); yüksek tehlikeli yerler dışında her sınıfta.
CIKMAZ = (15.0, 20.0)
#: Ek-5/B, çıkmaz koridor, yüksek tehlikeli yerler satırı.
CIKMAZ_YUKSEK = (10.0, 20.0)
MADDE = "BYKHY md. 4/i ve Ek-5/B (2009/15316 ve 2015/7401 sayılı BKK ile değişik)"
MADDE_YUKSEK = "BYKHY md. 4/i, 17 ve Ek-5/B (2009/15316 ve 2015/7401 sayılı BKK ile değişik)"


def _okunur(room: ext.RoomFacts, units: dict[str, ext.UnitFacts]) -> bool:
    """Koridor bu kuralın mı: konutun (ya da konutun eklentisinin) kendi holü değil."""
    unit = units.get(room.unit) if room.unit is not None else None
    if unit is not None and unit.kind == "annex" and unit.annex_of is not None:
        unit = units.get(unit.annex_of, unit)
    return not (unit is not None and unit.kind == "unit" and unit.dwelling)


def _sinif(
    room: ext.RoomFacts, units: dict[str, ext.UnitFacts], bina_sinifi: str | None
) -> tuple[str | None, str]:
    """Koridorun Ek-5/B sınıfı ve nereden okunduğu (bulgunun sözü): bölümünün; bölümü yoksa kendi adının, o da yoksa
    binanın en ağır sınıfı."""
    unit = units.get(room.unit) if room.unit is not None else None
    if unit is not None and unit.kind == "annex" and unit.annex_of is not None:
        unit = units.get(unit.annex_of, unit)
    if unit is not None and unit.kind == "unit" and unit.occupancy in UZAKLIK:
        return unit.occupancy, "bölümün niteliğinden"
    if room.occupancy in UZAKLIK:
        return room.occupancy, "mahalin adından"
    return bina_sinifi, "binanın en ağır sınıfı"


@ext.audit("cikmaz-koridor")
def cikmaz_koridor(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Türü ``corridor`` olan her mahalin çıkmazı (yürüyüşünün tek yön payı) Ek-5/B'nin çıkmaz koridor sütununa,
    projenin yağmurlama ayarının sınırıyla (15 m, yağmurlama sistemiyle 20 m; koridor yüksek tehlikeli yerinse
    yağmurlamasız 10 m): aşarsa hata. Yalnız konut bölümleri olan binada apartmanlar satırı ve 48/1'in muafiyeti; öbür
    binalarda koridorun sınıfının satırı. Konutun kendi holü, yürüyüşü olmayan ya da sınırı bulunamayan koridor
    okunmaz."""
    a = facts.project.architecture
    units = {u.id: u for u in a.units}
    homes = [u for u in a.units if u.kind == "unit"]
    residential = bool(homes) and all(u.dwelling for u in homes)
    if residential and bina(a).kacis_mesafesi_aranmaz:
        return []
    bilinen = [
        u.occupancy for u in a.units if u.kind == "unit" and not u.dwelling and u.occupancy in UZAKLIK
    ]
    bina_sinifi = min(bilinen, key=agir) if bilinen else None
    levels = kat_adlari(a)
    wet = yagmurlama(a)
    findings: list[ext.AuditFinding] = []
    for room in a.rooms:
        if room.kind != "corridor" or room.broken or room.escape is None or room.escape.exit is None:
            continue
        if not _okunur(room, units):
            continue
        walk = room.escape.exit
        dead_end = walk.one_way if walk.one_way is not None else walk.length
        sinif, kaynak = (None, "") if residential else _sinif(room, units, bina_sinifi)
        yuksek = sinif == "highHazard"
        limits, madde = (CIKMAZ_YUKSEK, MADDE_YUKSEK) if yuksek else (CIKMAZ, MADDE)
        if residential:
            who = "apartmanlar satırı"
        elif yuksek:
            who = f"yüksek tehlikeli yerler satırı, {kaynak}"
        else:
            who = "yüksek tehlikeli yerler dışında her sınıfın sınırı"
        decided = karar(dead_end, limits, wet)
        if decided is None:
            continue
        severity, _, verdict = decided
        findings.append(
            ext.AuditFinding(
                message=(
                    f"{mahal(room)} ({levels.get(room.level, room.level)}): çıkmaz koridor {metre(dead_end)} "
                    f"(koridorun en uzak noktasından bir çıkışa ya da iki yöne ayrıldığı noktaya); Ek-5/B'de çıkmaz "
                    f"koridor en çok {metre(limits[0])}, yağmurlama sistemiyle {metre(limits[1])} ({who}); {verdict} "
                    f"— {madde}."
                ),
                severity=severity,
                feature_id=room.uid,
            )
        )
    return findings
