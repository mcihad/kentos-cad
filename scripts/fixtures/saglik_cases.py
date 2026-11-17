#!/usr/bin/env python3
"""Sağlık yapılarının bağımsız durumları (docs/adr/8110): BYKHY 49/1-b (büyük yatılan katın kompartımanları) ve 49/2
(hastane koridorunun genişliği).

Her durumun beklenen sonucu yönetmeliğin metninden (mevzuat.gov.tr derlenmiş metni, SHA-256 d82d06d2…a7d9) elle
yazıldı; betik yalnız olguları kurar ve beklenenleri olduğu gibi yazar, kuralın kodunu çağırmaz. Yazar:
fixtures/standards/v1/saglik.json; `--check` karşılaştırır, yazmaz.

    python3 scripts/fixtures/saglik_cases.py [--check]
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/standards/v1/saglik.json"
TARIH = {"particulars": {"basvuru_tarihi": "02.07.2026"}}
_n = [0]


def uid():
    _n[0] += 1
    return f"00000000-0000-4000-8000-{_n[0]:012d}"


class Bina:
    """Bir binanın olguları: katlar, mahaller (adı, katı, alanı, kaçış merdiveni mi, koridorun en dar yeri), iki mahal
    arasındaki ayrımlar (duvarları ve kapıları, beyan edilen sınıflarıyla), katlar arası açıklıklar."""

    def __init__(self, uses, katlar=2, bolum=0, konut=0):
        self.b = {"storeys": [{"level": f"k{i:02d}", "name": "Zemin kat" if i == 0 else f"{i}. kat",
                               "elevation": 3.5 * i, "units": 0, "common": 1, "annexes": 0, "other": 0, "technical": 0,
                               "inhabited": 0, "elevators": 0, "stair": True} for i in range(katlar)],
                  "units": bolum, "dwellings": konut, "uses": uses, "spaces": [], "separations": [], "links": [],
                  "structureHeight": 3.5 * katlar}

    def mahal(self, ad, alan, kat=1, merdiven=False, en=None):
        u = uid()
        m = {"uid": u, "level": f"k{kat:02d}", "name": ad, "area": alan, "stair": merdiven}
        if en is not None:
            m["least"] = en
        self.b["spaces"].append(m)
        return u

    def ayrim(self, a, b, kat=1, sinif=None, kapi=None):
        """İki mahal arasındaki duvar (``sinif`` beyanıyla) ve varsa kapısı (``kapi`` beyanıyla)."""
        duvar = {"uid": uid(), "kind": "wall", "length": 4.0}
        if sinif:
            duvar["fireRating"] = sinif
        acik = []
        if kapi is not None:
            k = {"uid": uid(), "kind": "door", "length": 1.0}
            if kapi:
                k["fireRating"] = kapi
            acik.append(k)
        self.b["separations"].append({"level": f"k{kat:02d}", "rooms": [a, b], "open": 0.0, "walls": [duvar],
                                      "openings": acik})

    def bosluk(self, alt, ust, kat=0):
        """Alttaki ve üstteki mahalleri birleştiren döşeme boşluğu."""
        u = uid()
        self.b["links"].append({"uid": u, "kind": "hole", "below": f"k{kat:02d}", "above": f"k{kat + 1:02d}",
                                "roomsBelow": alt, "roomsAbove": ust, "area": 6.0, "stack": u})

    def kat(self, odalar, oda_alani, koridor_alani, kat=1, ad="Hasta odası", en=2.40, bolunmus=False):
        """Bir koridora açılan odalar; ``bolunmus``sa koridor iki yarıya ayrılır, aralarında EI 60 duvar ve EI2 60-C5
        kapı (iki kompartıman). Koridorların ve odaların kimlikleri."""
        yarilar = [self.mahal("Koridor", koridor_alani / 2, kat, en=en), self.mahal("Koridor", koridor_alani / 2, kat, en=en)] \
            if bolunmus else [self.mahal("Koridor", koridor_alani, kat, en=en)]
        if bolunmus:
            self.ayrim(yarilar[0], yarilar[1], kat, "EI 60", "EI2 60-C5")
        uids = []
        for i in range(odalar):
            oda = self.mahal(f"{ad} {i + 1}", oda_alani, kat)
            koridor = yarilar[0] if not bolunmus or i < odalar / 2 else yarilar[1]
            self.ayrim(oda, koridor, kat, None, "")
            uids.append(oda)
        return yarilar, uids


def kural(kat=None, koridor=None, kat_surum="9.9.2009", koridor_surum="19.12.2007"):
    return {"BYKHY 49/1-b": [kat, kat_surum], "BYKHY 49/2": [koridor, koridor_surum]}


def bulgu(code, has, state=None):
    out = {"code": code, "has": has}
    if state:
        out["state"] = state
    return out


KAT = "bykhy_patient_floor_compartments"
KORIDOR = "bykhy_hospital_corridor"


def durum(id_, neden, bina, bulgular, kurallar, izin=TARIH):
    return {"id": id_, "neden": neden, "permit": izin, "building": bina.b,
            "expect": {"findings": bulgular, "rules": kurallar}}


def durumlar():
    out = []
    b = Bina(["Hastane"])
    b.kat(8, 30.0, 120.0)
    out.append(durum("tek-kompartiman", "Hastanenin 360 m²'lik yatılan katı (8 hasta odası, koridor) tek kompartıman: "
                     "49/1-b '300 m2'den büyük olan yatılan katlarının her biri ... iki veya daha fazla yangın "
                     "kompartımanına ayrılır' → kalır.", b, [bulgu(KAT, ["360 m²", "tek yangın kompartımanı"])],
                     kural("failed", "passed")))
    b = Bina(["Hastane"])
    b.kat(8, 30.0, 120.0, bolunmus=True)
    out.append(durum("iki-kompartiman", "Aynı kat, koridor EI 60 duvar ve EI2 60-C5 kapıyla ikiye bölünmüş, her yarıda 4 "
                     "oda: iki kompartıman (180 m², 180 m²); 'en az yarısı büyüklüğünde' tek anlamla okunmuyor → uzman "
                     "incelemesi.", b, [bulgu(KAT, ["2 kompartıman", "180 m², 180 m²"], "expertReview")],
                     kural("expertReview", "passed")))
    b = Bina(["Hastane"])
    b.kat(7, 30.0, 70.0)
    out.append(durum("kucuk-kat", "280 m²'lik yatılan kat: 300 m²'den büyük değil → 49/1-b kapsam dışı.", b, [],
                     kural("outOfScope", "passed")))
    b = Bina(["Hastane"])
    b.kat(6, 30.0, 120.0)
    out.append(durum("tam-300", "Tam 300 m²'lik yatılan kat: '300 m2'den büyük' değil → kapsam dışı.", b, [],
                     kural("outOfScope", "passed")))
    b = Bina(["Hastane"])
    k = b.mahal("Koridor", 100.0, 1, en=2.4)
    for i in range(10):
        b.ayrim(b.mahal(f"Muayene {i + 1}", 50.0, 1), k, 1, None, "")
    out.append(durum("yatilmayan-kat", "600 m²'lik poliklinik katı, yatılan oda yok → 49/1-b kapsam dışı.", b, [],
                     kural("outOfScope", "passed")))
    b = Bina(["Huzurevi"])
    b.kat(12, 25.0, 60.0, ad="Oda")
    out.append(durum("huzurevi", "Huzurevinin 360 m²'lik odalı katı tek kompartıman: 'hastanelerin ve bakımevlerinin' → "
                     "kalır; huzurevi hastane değil → 49/2 kapsam dışı.", b, [bulgu(KAT, ["360 m²"])],
                     kural("failed", "outOfScope")))
    b = Bina(["Hastane"])
    (kor,), _ = b.kat(9, 30.0, 130.0)
    m = b.mahal("Merdiven", 20.0, 1, merdiven=True)
    b.ayrim(m, kor, 1, "EI 60", "EI2 60-C5")
    out.append(durum("merdiven-yuvasi", "400 m²'lik yatılan kat ve EI 60 ile ayrılmış kaçış merdiveni: merdivenin yuvası "
                     "katın kompartımanı sayılmaz → tek kompartıman, kalır.", b, [bulgu(KAT, ["tek yangın kompartımanı"])],
                     kural("failed", "passed")))
    b = Bina(["Hastane"])
    (kor,), _ = b.kat(9, 30.0, 130.0)
    b.ayrim(b.mahal("Elektrik odası", 12.0, 1), kor, 1, "EI 60", "EI2 60-C5")
    out.append(durum("teknik-oda", "Aynı kat ve EI 60 ile ayrılmış 12 m²'lik elektrik odası: iki kompartıman sayılır, "
                     "büyüklükleri iletide (400 m², 12 m²) → uzman incelemesi.", b,
                     [bulgu(KAT, ["400 m², 12 m²"], "expertReview")], kural("expertReview", "passed")))
    b = Bina(["Hastane"])
    (kor,), _ = b.kat(9, 30.0, 130.0)
    b.ayrim(b.mahal("Yatay tahliye alanı", 60.0, 1), kor, 1, "EI 60", "EI2 60-S200C5")
    out.append(durum("yatay-tahliye", "Katın korunumlu yatay tahliye alanı EI 60 ile ayrılmış: kompartıman sayılır; "
                     "2,8 m²/kişi hesabı çizimde yok → uzman incelemesi.", b,
                     [bulgu(KAT, ["2,8 m²/kişi"], "expertReview")], kural("expertReview", "passed")))
    b = Bina(["Konut"], bolum=6, konut=6)
    b.kat(8, 30.0, 120.0, ad="Yatak odası")
    out.append(durum("konut", "Konut binası: sağlık yapısı değil → ikisi de kapsam dışı.", b, [],
                     kural("outOfScope", "outOfScope")))
    b = Bina(["Otel"])
    b.kat(8, 30.0, 120.0, ad="Oda")
    out.append(durum("otel", "Otel: konaklama yapısı, sağlık yapısı değil → ikisi de kapsam dışı (50/1'in işi).", b, [],
                     kural("outOfScope", "outOfScope")))
    b = Bina([])
    b.kat(8, 30.0, 120.0, en=1.8)
    out.append(durum("adlardan", "Bina programında kullanım yok, mahaller Hasta odası: sağlık yapısı ve hastane "
                     "mahal adlarından → tek kompartıman kalır, 1,80 m'lik koridor kalır.", b,
                     [bulgu(KAT, ["360 m²"]), bulgu(KORIDOR, ["1,80 m", "2,00 m"])], kural("failed", "failed")))
    b = Bina(["Hastane"])
    b.kat(8, 30.0, 120.0)
    out.append(durum("basvuru-2008", "Başvuru 1.6.2008: 49/1-b'nin 9.9.2009 öncesi metni kural setinde yok → satır uzman "
                     "incelemesi; 49/2 2007'den → geçer.", b, [], kural("expertReview", "passed", None),
                     izin={"particulars": {"basvuru_tarihi": "01.06.2008"}}))
    b = Bina(["Hastane"])
    b.kat(8, 30.0, 120.0)
    out.append(durum("tarihsiz", "Başvuru tarihi yok: kural seti 9.9.2009'dan önceyi denemez, en yeni hâl → kalır.", b,
                     [bulgu(KAT, ["360 m²"])], kural("failed", "passed"), izin=None))
    b = Bina(["Hastane"], katlar=3)
    b.kat(8, 30.0, 120.0, kat=1)
    b.kat(8, 30.0, 120.0, kat=2, bolunmus=True)
    out.append(durum("iki-kat", "İki yatılan kat: 1. kat tek kompartıman (kalır), 2. kat iki kompartıman (uzman "
                     "incelemesi) → iki bulgu, satır kalır.", b,
                     [bulgu(KAT, ["1. kat", "tek yangın kompartımanı"]), bulgu(KAT, ["2. kat"], "expertReview")],
                     kural("failed", "passed")))
    b = Bina(["Hastane"], katlar=3)
    yarilar, _ = b.kat(8, 30.0, 120.0, kat=1, bolunmus=True)
    (ust,), _ = b.kat(4, 30.0, 60.0, kat=2)
    b.bosluk([yarilar[0]], [ust], kat=1)
    b.bosluk([yarilar[1]], [ust], kat=1)
    out.append(durum("bosluk-birlestirir", "1. katın iki yarısı EI 60 ile ayrılmış, ama ikisi de döşeme boşluğuyla 2. "
                     "katın aynı koridoruna açılıyor: katlar arası açıklık kompartımanları birleştirir → tek kompartıman, "
                     "kalır (2. kat 180 m², kapsam dışı).", b, [bulgu(KAT, ["1. kat", "tek yangın kompartımanı"])],
                     kural("failed", "passed")))
    b = Bina(["Hastane"])
    b.kat(6, 30.0, 60.0, en=1.8)
    out.append(durum("koridor-dar", "Hastane koridorunun en dar yeri 1,80 m: 49/2 'koridor genişlikleri 2 m'den az "
                     "olamaz' → kalır.", b, [bulgu(KORIDOR, ["1,80 m", "2,00 m"])], kural("outOfScope", "failed")))
    b = Bina(["Hastane"])
    b.kat(6, 30.0, 60.0, en=2.0)
    out.append(durum("koridor-tam-2", "En dar yeri tam 2,00 m: 2 m'den az değil → geçer.", b, [],
                     kural("outOfScope", "passed")))
    b = Bina(["Hastane"])
    b.kat(6, 30.0, 60.0, en=1.999)
    out.append(durum("koridor-1999", "En dar yeri 1,999 m: 2 m'den az → kalır; ileti milimetreyi gösterir.", b,
                     [bulgu(KORIDOR, ["1,999 m"])], kural("outOfScope", "failed")))
    b = Bina(["Hastane"])
    k = b.mahal("Koridor", 60.0, 1)
    b.ayrim(b.mahal("Hasta odası", 30.0, 1), k, 1, None, "")
    out.append(durum("koridor-olculemedi", "Koridorun sınırı bulunamamış, en dar yeri ölçülmemiş → eksik veri.", b,
                     [bulgu(KORIDOR, ["ölçülemedi"], "missingData")], kural("outOfScope", "missingData")))
    b = Bina(["Hastane"])
    h = b.mahal("Hol", 40.0, 1, en=1.5)
    b.ayrim(b.mahal("Hasta odası", 30.0, 1), h, 1, None, "")
    out.append(durum("hol-sayilmaz", "1,50 m'lik Hol, adı Koridor olan mahal yok: 49/2 koridoru söyler → kapsam dışı.", b,
                     [], kural("outOfScope", "outOfScope")))
    b = Bina(["Bakımevi"])
    b.kat(6, 30.0, 60.0, ad="Oda", en=1.6)
    out.append(durum("bakimevi-koridor", "Bakımevinin 1,60 m'lik koridoru: 49/2 'hastanelerde' → kapsam dışı.", b, [],
                     kural("outOfScope", "outOfScope")))
    b = Bina(["Hastane"])
    k = b.mahal("Hasta koridoru", 60.0, 1, en=1.9)
    b.ayrim(b.mahal("Hasta odası", 30.0, 1), k, 1, None, "")
    out.append(durum("hasta-koridoru", "Adı “Hasta koridoru”, en dar yeri 1,90 m → kalır.", b,
                     [bulgu(KORIDOR, ["Hasta koridoru", "1,90 m"])], kural("outOfScope", "failed")))
    b = Bina(["Klinik"])
    b.kat(8, 30.0, 120.0, en=1.8)
    out.append(durum("klinik", "Kullanımı Klinik: sağlık yapısı (11/1-b) → tek kompartımanlı 360 m² kat kalır; hastane "
                     "değil → 49/2 kapsam dışı.", b, [bulgu(KAT, ["360 m²"])], kural("failed", "outOfScope")))
    b = Bina(["Hastane", "Konut"])
    b.kat(6, 30.0, 60.0, en=1.7)
    out.append(durum("karma", "Hastane ve lojman: hastane → 1,70 m'lik koridor kalır.", b,
                     [bulgu(KORIDOR, ["1,70 m"])], kural("outOfScope", "failed")))
    return out


def belge():
    return {
        "format": "kentos.regulationCases", "version": 1,
        "about": ("Sağlık yapılarının bağımsız durumları (docs/adr/8110): beklenen her sonuç Binaların Yangından Korunması "
                  "Hakkında Yönetmelik 49/1-b'nin (RG 9.9.2009/27344) ve 49/2'nin (RG 19.12.2007/26735) metninden elle "
                  "çıkarıldı; kuralın kodundan üretilmedi (scripts/fixtures/saglik_cases.py olguları kurar, beklenenleri "
                  "olduğu gibi yazar)."),
        "cases": durumlar(),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    metin = json.dumps(belge(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != metin:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(belge()['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(metin, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(belge()['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
