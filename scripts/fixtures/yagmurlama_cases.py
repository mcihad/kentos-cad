#!/usr/bin/env python3
"""Yağmurlama zorunluluğunun bağımsız durumları (docs/adr/8108): BYKHY 96/2'nin a–e bentleri.

Her durumun beklenen sonucu yönetmeliğin metninden (mevzuat.gov.tr derlenmiş metni, SHA-256 d82d06d2…a7d9) elle
yazıldı; betik yalnız olguları kurar ve beklenenleri olduğu gibi yazar, kuralın kodunu çağırmaz. Yazar:
fixtures/standards/v1/yagmurlama.json; `--check` karşılaştırır, yazmaz.

    python3 scripts/fixtures/yagmurlama_cases.py [--check]
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/standards/v1/yagmurlama.json"
TARIH = {"particulars": {"basvuru_tarihi": "02.07.2026"}}
_n = [0]


def uid():
    _n[0] += 1
    return f"00000000-0000-4000-8000-{_n[0]:012d}"


class Bina:
    def __init__(self, uses, katlar, yukseklik, yagmurlama=False, bolum=0, konut=0):
        self.b = {"storeys": [{"level": f"k{i:02d}", "name": "Zemin kat" if i == 0 else f"{i}. kat",
                               "elevation": 3.0 * i, "units": 0, "common": 1, "annexes": 0, "other": 0, "technical": 0,
                               "inhabited": 0, "elevators": 0, "stair": True} for i in range(katlar)],
                  "units": bolum, "dwellings": konut, "uses": uses, "spaces": [], "structureHeight": yukseklik}
        if yagmurlama:
            self.b["sprinklers"] = True

    def mahal(self, ad, alan, kat=0):
        u = uid()
        self.b["spaces"].append({"uid": u, "level": f"k{kat:02d}", "name": ad, "area": alan, "stair": False})
        return u

    def odalar(self, n, kat_sayisi):
        for i in range(n):
            self.mahal(f"Oda {i + 1}", 20.0, 1 + i % max(kat_sayisi - 1, 1))


def kural(durum, surum="9.9.2009"):
    return {"BYKHY 96/2": [durum, surum]}


def bulgu(has, state=None):
    out = {"code": "bykhy_sprinklers_required", "has": has}
    if state:
        out["state"] = state
    return out


def durum(id_, neden, bina, bulgular, kurallar, izin=TARIH):
    return {"id": id_, "neden": neden, "permit": izin, "building": bina.b,
            "expect": {"findings": bulgular, "rules": kurallar}}


def buro(katlar, yukseklik, **k):
    b = Bina(["Büro"], katlar, yukseklik, **k)
    for i in range(katlar):
        b.mahal("Ofis", 150.0, i)
    return b


def durumlar():
    out = []
    out.append(durum("buro-yuksek", "11 katlı büro, yapı yüksekliği 33 m, yağmurlama yok: a) 'yapı yüksekliği 30.50 m'den "
                     "fazla olan konut haricindeki bütün binalarda' → kalır.", buro(11, 33.0),
                     [bulgu(["30,50 m", "33,00 m", "BYKHY 96/2-a"])], kural("failed")))
    out.append(durum("buro-yuksek-beyanli", "Aynı büro, yağmurlama beyan edilmiş → geçer.", buro(11, 33.0, yagmurlama=True),
                     [], kural("passed")))
    out.append(durum("buro-alcak", "8 katlı büro, 24 m: hiçbir bent istemiyor → kapsam dışı.", buro(8, 24.0), [],
                     kural("outOfScope")))
    b = Bina(["Konut"], 13, 40.0, bolum=24, konut=24)
    out.append(durum("konut-40", "40 m'lik konut: a) konut haricindekiler, b) 51,50 m'yi geçen konut → kapsam dışı.", b, [],
                     kural("outOfScope")))
    b = Bina(["Konut"], 18, 54.0, bolum=34, konut=34)
    out.append(durum("konut-54", "54 m'lik konut: b) 'yapı yüksekliği 51.50 m'yi geçen konutlarda' → kalır.", b,
                     [bulgu(["konut", "51,50 m", "BYKHY 96/2-b"])], kural("failed")))
    b = Bina(["Konut", "Ticaret"], 11, 32.0, bolum=20, konut=18)
    out.append(durum("karma-32", "Konut ve ticaret karması 32 m: konut haricindeki bina sayılır (sıkı olan) → a) kalır.", b,
                     [bulgu(["konut haricindeki", "32,00 m"])], kural("failed")))
    b = Bina(["Konut"], 4, 12.0, bolum=6, konut=6)
    b.mahal("Otopark", 650.0)
    out.append(durum("otopark-650", "Konut binasının 650 m²'lik kapalı otoparkı: c) 'alanlarının toplamı 600 m2'den büyük "
                     "olan kapalı otoparklarda' → kalır.", b, [bulgu(["650 m²", "600 m²", "BYKHY 96/2-c"])],
                     kural("failed")))
    b = Bina(["Konut"], 4, 12.0, bolum=6, konut=6)
    b.mahal("Otopark", 550.0)
    out.append(durum("otopark-550", "550 m²'lik otopark → kapsam dışı.", b, [], kural("outOfScope")))
    b = Bina(["Konut"], 4, 12.0, bolum=6, konut=6)
    b.mahal("Otopark", 350.0)
    b.mahal("Garaj", 300.0, 1)
    out.append(durum("otopark-toplam", "İki otopark mahali 350 + 300 m²: bent alanların toplamını sayar → kalır.", b,
                     [bulgu(["650 m²"])], kural("failed")))
    b = Bina(["Otel"], 4, 12.0)
    b.odalar(110, 4)
    out.append(durum("otel-110-oda", "Dört katlı otel, 110 oda: ç) 'yatılan oda sayısı 100'ü … geçen otellerde' → kalır.",
                     b, [bulgu(["110 yatılan oda > 100", "BYKHY 96/2-ç"])], kural("failed")))
    b = Bina(["Otel"], 4, 12.0)
    b.odalar(40, 4)
    out.append(durum("otel-40-oda", "Dört katlı otel, 40 oda: yatak sayısı 200'ü geçerse gerekir, çizimde yok → eksik veri.",
                     b, [bulgu(["40 yatılan oda", "yatak sayısı çizimde yok"], "missingData")], kural("missingData")))
    b = Bina(["Otel"], 1, 3.0)
    for i in range(40):
        b.mahal(f"Oda {i + 1}", 20.0)
    out.append(durum("otel-tek-katli", "Tek katlı otel: ç) 'birden fazla katlı bir bina' → kapsam dışı.", b, [],
                     kural("outOfScope")))
    b = Bina(["Otel"], 8, 24.0)
    b.odalar(30, 8)
    out.append(durum("otel-24", "Sekiz katlı otel, 24 m: ç) 'yapı yüksekliği 21.50 m'den fazla olan bütün yataklı "
                     "tesislerde' → kalır.", b, [bulgu(["yataklı tesis", "21,50 m"])], kural("failed")))
    b = Bina(["Hastane"], 8, 24.0)
    b.mahal("Hasta odası", 30.0, 1)
    out.append(durum("hastane-24", "24 m'lik hastane yataklı tesistir → ç) kalır.", b, [bulgu(["yataklı tesis"])],
                     kural("failed")))
    b = Bina(["Hastane"], 5, 15.0)
    b.mahal("Hasta odası", 30.0, 1)
    out.append(durum("hastane-15", "15 m'lik hastane: ç'nin oda sayısı oteller, yurtlar, pansiyonlar, misafirhaneler "
                     "içindir; yükseklik 21,50 m'nin altında → kapsam dışı.", b, [], kural("outOfScope")))
    b = Bina(["Alışveriş merkezi"], 2, 8.0)
    b.mahal("Mağaza", 1300.0, 0)
    b.mahal("Mağaza", 1200.0, 1)
    out.append(durum("avm-2500", "İki katlı alışveriş merkezi, 2500 m²: d) 'toplam alanı 2000 m2'nin üzerinde olan katlı "
                     "mağazalarda, alışveriş …' → kalır.", b, [bulgu(["2500 m² > 2000 m²", "BYKHY 96/2-d"])],
                     kural("failed")))
    b = Bina(["Alışveriş merkezi"], 1, 5.0)
    b.mahal("Mağaza", 2500.0)
    out.append(durum("avm-tek-kat", "Tek katlı 2500 m² mağaza: 'katlı' değil → kapsam dışı.", b, [], kural("outOfScope")))
    b = Bina(["Alışveriş merkezi"], 2, 8.0)
    b.mahal("Mağaza", 900.0, 0)
    b.mahal("Mağaza", 900.0, 1)
    out.append(durum("avm-1800", "İki katlı 1800 m² → kapsam dışı.", b, [], kural("outOfScope")))
    b = Bina(["Konut", "Ticaret"], 4, 12.0, bolum=10, konut=8)
    b.mahal("Mağaza", 1200.0, 0)
    b.mahal("Mağaza", 1200.0, 1)
    b.mahal("Salon", 120.0, 2)
    out.append(durum("karma-dukkan-iki-kat", "Karma binada iki kata yayılan 2400 m² mağaza: bent mağazanın kendi alanını "
                     "sayar → kalır.", b, [bulgu(["2400 m²"])], kural("failed")))
    b = Bina(["Konut", "Ticaret"], 4, 12.0, bolum=10, konut=8)
    b.mahal("Mağaza", 2400.0, 0)
    out.append(durum("karma-dukkan-tek-kat", "Karma binanın zemin katında 2400 m² mağaza: katlı değil → kapsam dışı.", b, [],
                     kural("outOfScope")))
    b = Bina(["LPG dolum tesisi"], 1, 6.0)
    b.mahal("Dolum salonu", 1200.0)
    out.append(durum("lpg", "1200 m²'lik LPG dolum tesisi: e) 'kolay alevlenici ve parlayıcı madde üretilen veya "
                     "bulundurulan yapılarda'; ne bulundurduğu çizimde yok → uzman incelemesi.", b,
                     [bulgu(["parlayıcı madde", "1200 m² > 1000 m²", "BYKHY 96/2-e"], "expertReview")],
                     kural("expertReview")))
    b = Bina(["LPG dolum tesisi"], 1, 6.0)
    b.mahal("Dolum salonu", 600.0)
    out.append(durum("lpg-kucuk", "600 m² → kapsam dışı.", b, [], kural("outOfScope")))
    b = buro(11, 33.0)
    b.mahal("Otopark", 700.0)
    out.append(durum("iki-bent", "33 m'lik büronun 700 m² otoparkı: a) ve c) ikisi de ister → iki bulgu.", b,
                     [bulgu(["BYKHY 96/2-a"]), bulgu(["700 m²", "BYKHY 96/2-c"])], kural("failed")))
    out.append(durum("basvuru-2008", "33 m'lik büro, başvuru 1.6.2008: bentlerin 9.9.2009 öncesi metni kural setinde yok → "
                     "satır uzman incelemesi.", buro(11, 33.0), [], kural("expertReview", None),
                     izin={"particulars": {"basvuru_tarihi": "01.06.2008"}}))
    out.append(durum("tarihsiz", "Başvuru tarihi yok: kural seti 9.9.2009'dan önceyi denemez, en yeni hâl → a) kalır.",
                     buro(11, 33.0), [bulgu(["BYKHY 96/2-a"])], kural("failed"), izin=None))
    b = Bina(["Sinema"], 2, 9.0)
    b.mahal("Salon", 1100.0, 0)
    b.mahal("Salon", 1100.0, 1)
    out.append(durum("sinema", "İki katlı 2200 m² sinema: eğlence yeri (15/1-b) → d) kalır.", b, [bulgu(["2200 m²"])],
                     kural("failed")))
    return out


def belge():
    return {
        "format": "kentos.regulationCases", "version": 1,
        "about": ("Yağmurlama zorunluluğunun bağımsız durumları (docs/adr/8108): beklenen her sonuç Binaların Yangından "
                  "Korunması Hakkında Yönetmelik 96/2'nin metninden (a, ç, e RG 19.12.2007/26735; b, c, d RG "
                  "9.9.2009/27344) elle çıkarıldı; kuralın kodundan üretilmedi (scripts/fixtures/yagmurlama_cases.py "
                  "olguları kurar, beklenenleri olduğu gibi yazar)."),
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
