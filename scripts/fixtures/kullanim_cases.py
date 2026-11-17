#!/usr/bin/env python3
"""Kullanım sınıfına özel kaçış hükümlerinin bağımsız durumları (docs/adr/8107): BYKHY 49/1-a, 50/1-a, 50/1-b ve 52.

Her durumun beklenen sonucu yönetmeliğin metninden (mevzuat.gov.tr derlenmiş metni, SHA-256 d82d06d2…a7d9) elle
yazıldı; betik yalnız olguları kurar ve beklenenleri olduğu gibi yazar, kuralın kodunu çağırmaz. Yazar:
fixtures/standards/v1/kullanim.json; `--check` karşılaştırır, yazmaz.

    python3 scripts/fixtures/kullanim_cases.py [--check]
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/standards/v1/kullanim.json"
TARIH = {"particulars": {"basvuru_tarihi": "02.07.2026"}}
V07, V09 = "19.12.2007", "9.9.2009"
KURALLAR = ("BYKHY 49/1-a", "BYKHY 50/1-a", "BYKHY 50/1-b", "BYKHY 52")
_n = [0]


def uid():
    _n[0] += 1
    return f"00000000-0000-4000-8000-{_n[0]:012d}"


def kat(i, kot):
    return {"level": f"k{i:02d}", "name": "Zemin kat" if i == 0 else f"{i}. kat", "elevation": kot, "units": 0,
            "common": 1, "annexes": 0, "other": 0, "technical": 0, "inhabited": 0, "elevators": 0, "stair": True}


class Bina:
    def __init__(self, uses, katlar=3, kat_yuksekligi=3.5, yukseklik=None, bolum=0, konut=0):
        self.b = {"storeys": [kat(i, i * kat_yuksekligi) for i in range(katlar)], "units": bolum, "dwellings": konut,
                  "uses": uses, "rooms": [], "spaces": [], "doors": [], "separations": [],
                  "structureHeight": yukseklik if yukseklik is not None else katlar * kat_yuksekligi}

    def mahal(self, seviye, ad, kisi=None, merdiven=False, alan=20.0):
        u = uid()
        self.b["spaces"].append({"uid": u, "level": seviye, "name": ad, "area": alan, "stair": merdiven})
        if kisi is not None:
            self.b["rooms"].append({"uid": u, "level": seviye, "name": ad, "persons": kisi})
        return u

    def merdivenler(self, n, katlar=None):
        for k in self.b["storeys"]:
            if katlar is None or k["level"] in katlar:
                for j in range(n):
                    self.mahal(k["level"], "Merdiven evi" if j == 0 else f"Merdiven evi {j + 1}", merdiven=True)

    def kapi(self, seviye, a, b, sinif=None):
        u = uid()
        k = {"uid": u, "level": seviye, "between": [a, b], "operation": "swing"}
        if sinif:
            k["fireRating"] = sinif
        self.b["doors"].append(k)
        return u

    def duvar(self, seviye, a, b, sinif=None):
        d = {"uid": uid(), "kind": "wall", "length": 4.0}
        if sinif:
            d["fireRating"] = sinif
        self.b["separations"].append({"level": seviye, "rooms": [a, b], "walls": [d], "openings": [], "open": 0.0})


def satirlar(**durum):
    """Dört maddenin beklenen satırları: verilmeyen kapsam dışıdır."""
    out = {}
    for k, s in zip(KURALLAR, ("m49", "m50a", "m50b", "m52")):
        st = durum.get(s, "outOfScope")
        surum = durum.get(s + "_surum", V09 if s == "m52" else V07)
        out[k] = [st, surum]
    return out


def durum(id_, neden, bina, bulgular, kurallar, izin=TARIH):
    return {"id": id_, "neden": neden, "permit": izin, "building": bina.b,
            "expect": {"findings": bulgular, "rules": kurallar}}


def buro(katlar, merdiven, yukseklik=None, kisi=20, uses=("Büro",)):
    b = Bina(list(uses), katlar=katlar, yukseklik=yukseklik)
    for k in b.b["storeys"]:
        b.mahal(k["level"], "Ofis", kisi=kisi, alan=kisi * 10.0)
    b.merdivenler(merdiven)
    return b


def otel(katlar=2, merdiven=2, duvar="EI 60", kapi="EI2 30-C5", yukseklik=None, uses=("Otel",), oda="Oda 101"):
    b = Bina(list(uses), katlar=katlar, kat_yuksekligi=3.0, yukseklik=yukseklik)
    for k in b.b["storeys"]:
        if k["elevation"] == 0.0:
            b.mahal(k["level"], "Resepsiyon", kisi=10)
            continue
        o = b.mahal(k["level"], oda, kisi=2, alan=25.0)
        c = b.mahal(k["level"], "Koridor", alan=30.0)
        b.duvar(k["level"], o, c, duvar)
        b.kapi(k["level"], o, c, kapi)
    b.merdivenler(merdiven)
    return b


def hastane(kisi, kapilar):
    b = Bina(["Hastane"], katlar=2)
    o = b.mahal("k01", "Hasta odası", kisi=kisi, alan=kisi * 20.0)
    c = b.mahal("k01", "Koridor", alan=40.0)
    for _ in range(kapilar):
        b.kapi("k01", o, c)
    b.merdivenler(2)
    return b


def bulgu(code, has, state=None):
    out = {"code": code, "has": has}
    if state:
        out["state"] = state
    return out


def durumlar():
    out = []
    # 52: fabrika, imalathane, mağaza, dükkân, depo, büro ve ayakta tedavi merkezi.
    out.append(durum("buro-iki-merdiven", "Üç katlı büro, her katta iki kaçış merdiveni: 52 'en az 2 bağımsız kaçış "
                     "merdiveni' → geçer.", buro(3, 2), [], satirlar(m52="passed")))
    b = buro(8, 1, yukseklik=28.0)
    out.append(durum("buro-tek-merdiven-yuksek", "Sekiz katlı büro (yapı yüksekliği 28 m), tek merdiven: 52/a 'yapı "
                     "yüksekliğinin 21.50 m'den az olması' tutmaz → zemin üstündeki yedi katın her biri kalır.", b,
                     [bulgu("bykhy_single_stair", [f"{i}. kat", "1 kaçış merdiveni", "21,50 m'den az değil (a)",
                                                   "BYKHY 52"]) for i in range(1, 8)],
                     satirlar(m52="failed")))
    b = buro(3, 1, kisi=20)
    b.b["rooms"][1]["persons"] = 60
    out.append(durum("buro-tek-merdiven-kalabalik", "Üç katlı büro, tek merdiven, 1. katta 60 kişi: 52/b 'bir kattaki "
                     "kullanıcı sayısının 50 kişiden az olması' tutmaz → iki üst katın her biri kalır.", b,
                     [bulgu("bykhy_single_stair", ["1. kat", "60 kişi, 50'den az değil (b)"]),
                      bulgu("bykhy_single_stair", ["2. kat", "60 kişi, 50'den az değil (b)"])],
                     satirlar(m52="failed")))
    out.append(durum("buro-tek-merdiven-kosullar", "Üç katlı büro (10,5 m), tek merdiven, katlarda 20 kişi: (a) ve (b) "
                     "tutar; (c) kaçış uzaklıkları, (ç) yanmaz ürünler ve (d) kolay alevlenen madde çizimden "
                     "kanıtlanmaz → uzman incelemesi.", buro(3, 1),
                     [bulgu("bykhy_single_stair", ["1. kat", "çizimden kanıtlanmaz"], "expertReview"),
                      bulgu("bykhy_single_stair", ["2. kat", "çizimden kanıtlanmaz"], "expertReview")],
                     satirlar(m52="expertReview")))
    out.append(durum("magaza-tek-katli", "Tek katlı mağaza: zemin kattan başka mahalli kat yok, 52'nin merdiven "
                     "sayısı aranmaz (çıkışlarını 39/2 ve 32/8 söyler).", buro(1, 0, uses=("Mağaza",)), [],
                     satirlar(m52="outOfScope")))
    out.append(durum("depo-programdan", "Bina programı 'Depo' yazıyor (16/1-a), iki kat, tek merdiven, (a) ve (b) "
                     "tutar → uzman incelemesi.", buro(2, 1, kisi=10, uses=("Depo",)),
                     [bulgu("bykhy_single_stair", ["1. kat", "çizimden kanıtlanmaz"], "expertReview")],
                     satirlar(m52="expertReview")))
    b = buro(3, 2, uses=())
    b.b["units"] = 3
    out.append(durum("program-yok-ofis-adlari", "Bina programında kullanım yok, konut bölümü yok, mahaller 'Ofis': büro "
                     "binası okunur; her katta iki merdiven → geçer.", b, [], satirlar(m52="passed")))
    b = Bina([], katlar=3, bolum=2, konut=2)
    for k in b.b["storeys"]:
        b.mahal(k["level"], "Depo", kisi=1)
    b.merdivenler(1)
    out.append(durum("konut-depo-adi", "Bütün bölümleri konut olan binanın 'Depo' adlı mahalleri binayı depo yapmaz "
                     "(16/2: 50 m²'den küçük depolama bölümü esas binanın parçası) → dört madde kapsam dışı.", b, [],
                     satirlar()))
    out.append(durum("basvuru-2009-oncesi", "Tek merdivenli üç katlı büro, başvuru 1.6.2009: 52'nin 9.9.2009 öncesi "
                     "metni kural setinde yok → satır uzman incelemesi, bulgu yok.", buro(3, 1, yukseklik=28.0), [],
                     satirlar(m52="expertReview", m52_surum=None),
                     izin={"particulars": {"basvuru_tarihi": "01.06.2009"}}))
    out.append(durum("ayakta-tedavi", "Bina programı 'Ayakta tedavi merkezi': 52'nin binası; iki kat, tek merdiven, (a) "
                     "ve (b) tutar → uzman incelemesi.", buro(2, 1, kisi=10, uses=("Ayakta tedavi merkezi",)),
                     [bulgu("bykhy_single_stair", ["1. kat", "çizimden kanıtlanmaz"], "expertReview")],
                     satirlar(m52="expertReview")))
    # 50: oteller, moteller ve yatakhaneler.
    out.append(durum("otel-tutuyor", "İki katlı otel: yatak odası iç koridordan EI 60 duvarla ayrılmış (50/1-a), "
                     "koridora açılan kapı EI2 30-C5 (50/1-b), katta iki merdiven → ikisi de geçer.", otel(), [],
                     satirlar(m50a="passed", m50b="passed")))
    out.append(durum("otel-duvar-30", "Yatak odası ile iç koridor arası EI 30: 50/1-a 'en az 60 dakika' → kalır.",
                     otel(duvar="EI 30"), [bulgu("bykhy_hotel_room_wall", ["Oda 101", "30 dakika < 60 dakika",
                                                                            "BYKHY 50/1-a"])],
                     satirlar(m50a="failed", m50b="passed")))
    out.append(durum("otel-duvar-beyansiz", "Yatak odası ile koridor arasındaki duvarın yangın sınıfı beyan edilmemiş "
                     "→ eksik veri.", otel(duvar=None),
                     [bulgu("bykhy_hotel_room_wall", ["yangın sınıfı beyan edilmemiş"], "missingData")],
                     satirlar(m50a="missingData", m50b="passed")))
    out.append(durum("otel-kapi-kapanmiyor", "Koridora açılan kapı EI 30, kendiliğinden kapanmıyor: 50/1-b "
                     "'kendiliğinden kapatan düzenekler' → kalır.", otel(kapi="EI 30"),
                     [bulgu("bykhy_hotel_room_door", ["Oda 101", "kendiliğinden kapanmıyor", "BYKHY 50/1-b"])],
                     satirlar(m50a="passed", m50b="failed")))
    out.append(durum("otel-kapi-15dk", "Koridora açılan kapı EI2 15-C5: 50/1-b 'en az 30 dakika' → kalır.",
                     otel(kapi="EI2 15-C5"), [bulgu("bykhy_hotel_room_door", ["15 dakika < 30 dakika"])],
                     satirlar(m50a="passed", m50b="failed")))
    out.append(durum("otel-kapi-beyansiz", "Koridora açılan kapının sınıfı yok → eksik veri.", otel(kapi=None),
                     [bulgu("bykhy_hotel_room_door", ["yangın sınıfı beyan edilmemiş"], "missingData")],
                     satirlar(m50a="passed", m50b="missingData")))
    out.append(durum("otel-tek-cikis-yuksek", "Altı katlı otel (18 m), her katta tek merdiven: kat sayısı ikiden "
                     "fazla, 2 çıkış gerekir; 'yatak 20'den az ve yapı yüksekliği 15.50 m'den az' istisnası 18 m'de "
                     "uygulanmaz → yatak odalı beş katın her biri kalır.", otel(katlar=6, merdiven=1, yukseklik=18.0),
                     [bulgu("bykhy_hotel_storey_exits", [f"{i}. kat", "1 çıkış", "15,50 m'nin altında değil"])
                      for i in range(1, 6)], satirlar(m50a="failed", m50b="passed")))
    out.append(durum("otel-tek-cikis-alcak", "Dört katlı otel (12 m), tek merdiven: istisna yatak sayısına ve merdivenin "
                     "korunumuna bağlı, ikisi de çizimde yok → üç yatak odalı kat eksik veri.",
                     otel(katlar=4, merdiven=1, yukseklik=12.0),
                     [bulgu("bykhy_hotel_storey_exits", [f"{i}. kat", "yatak sayısı ve merdivenin korunumu"],
                            "missingData") for i in range(1, 4)], satirlar(m50a="missingData", m50b="passed")))
    out.append(durum("otel-iki-katli-tek-cikis", "İki katlı otel, tek merdiven: 20'den fazla yatakta her katta 2 çıkış "
                     "gerekir, yatak sayısı çizimde yok → eksik veri.", otel(merdiven=1),
                     [bulgu("bykhy_hotel_storey_exits", ["1. kat", "toplam yatak 20'den fazlaysa"], "missingData")],
                     satirlar(m50a="missingData", m50b="passed")))
    out.append(durum("yurt-programdan", "Bina programı 'Öğrenci yurdu' (10: öğrenci yurtları konaklama amaçlı), 'Yatak "
                     "odası' koridordan EI 60, kapı EI2 30-C5 → geçer.", otel(uses=("Öğrenci yurdu",),
                                                                              oda="Yatak odası"), [],
                     satirlar(m50a="passed", m50b="passed")))
    # 49: sağlık yapıları.
    out.append(durum("hastane-tek-kapi", "Hastanede 18 kişilik hasta odası tek kapılı: 49/1-a 'kullanıcı yükü 15 kişiyi "
                     "aşan … 2 kapı' → kalır.", hastane(18, 1),
                     [bulgu("bykhy_patient_room_doors", ["Hasta odası", "18 kişi", "1 kapısı var", "BYKHY 49/1-a"])],
                     satirlar(m49="failed")))
    out.append(durum("hastane-iki-kapi", "18 kişilik hasta odasının iki kapısı var → geçer (birbirinden uzaklığı "
                     "metinde ölçü değil, idarenin).", hastane(18, 2), [], satirlar(m49="passed")))
    out.append(durum("hastane-kucuk-oda", "İki kişilik hasta odası: 15 kişiyi aşmıyor → 49/1-a kapsam dışı.",
                     hastane(2, 1), [], satirlar(m49="outOfScope")))
    out.append(durum("konut-kapsam-disi", "Bina programı 'Konut': dört madde de kapsam dışı.",
                     buro(3, 1, uses=("Konut",)), [], satirlar()))
    b = buro(8, 1, yukseklik=28.0)
    out.append(durum("tarihsiz-buro", "Başvuru tarihi yok: kural seti bütün maddelerin bilindiği ilk günden (9.9.2009) "
                     "önceki günleri denemez (52'nin önceki metni kural setinde yok); o günden sonra tek hâl var, sonuç "
                     "güne bağlı değil → tek merdivenli sekiz katlı büronun yedi katı en yeni hâlle kalır.", b,
                     [bulgu("bykhy_single_stair", [f"{i}. kat", "21,50 m'den az değil (a)"]) for i in range(1, 8)],
                     satirlar(m52="failed"), izin=None))
    return out


def belge():
    return {
        "format": "kentos.regulationCases", "version": 1,
        "about": ("Kullanım sınıfına özel kaçış hükümlerinin bağımsız durumları (docs/adr/8107): beklenen her sonuç "
                  "Binaların Yangından Korunması Hakkında Yönetmelik 49/1-a, 50/1-a, 50/1-b (RG 19.12.2007/26735, "
                  "değişmedi) ve 52'nin (RG 9.9.2009/27344) metninden elle çıkarıldı; kuralın kodundan üretilmedi "
                  "(scripts/fixtures/kullanim_cases.py olguları kurar, beklenenleri olduğu gibi yazar)."),
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
