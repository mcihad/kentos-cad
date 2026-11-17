#!/usr/bin/env python3
"""Kazan dairesi ve yakıt deposunun bağımsız durumları (docs/adr/8113): BYKHY 54/2, 54/4, 54/5 ve 56/1.

Her durumun beklenen sonucu yönetmeliğin metninden (mevzuat.gov.tr derlenmiş metni, SHA-256 d82d06d2…a7d9) elle
yazıldı; betik yalnız olguları kurar ve beklenenleri olduğu gibi yazar, kuralın kodunu çağırmaz. Yazar:
fixtures/standards/v1/kazan.json; `--check` karşılaştırır, yazmaz.

    python3 scripts/fixtures/kazan_cases.py [--check]
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/standards/v1/kazan.json"
TARIH = {"particulars": {"basvuru_tarihi": "02.07.2026"}}
AYRIM, KAPI, CIKIS, SINIF, YAKIT = ("bykhy_boiler_room_separation", "bykhy_boiler_room_door",
                                     "bykhy_boiler_room_exits", "bykhy_boiler_room_door_rating",
                                     "bykhy_fuel_store_separation")
KAPI_IYI = "EI2 90-S200C5"
_n = [0]


def uid():
    _n[0] += 1
    return f"00000000-0000-4000-8000-{_n[0]:012d}"


class Bina:
    """Bodrum katında bir koridor ve mahaller: kazan dairesi, merdiven, depo …; aralarındaki ayrımlar ve kapılar."""

    def __init__(self):
        self.b = {"storeys": [{"level": "b01", "name": "Bodrum kat", "elevation": -3.0, "units": 0, "common": 1,
                               "annexes": 0, "other": 0, "technical": 1, "inhabited": 0, "elevators": 0, "stair": True},
                              {"level": "k00", "name": "Zemin kat", "elevation": 0.0, "units": 2, "common": 1,
                               "annexes": 0, "other": 0, "technical": 0, "inhabited": 2, "elevators": 0,
                               "stair": True}],
                  "units": 4, "dwellings": 4, "uses": ["Konut"], "spaces": [], "separations": [], "doors": [],
                  "structureHeight": 9.0}
        self.koridor = self.mahal("Koridor", 20.0)

    def mahal(self, ad, net, merdiven=False):
        u = uid()
        m = {"uid": u, "level": "b01", "name": ad, "area": round(net * 1.08, 2), "stair": merdiven}
        if net is not None:
            m["net"] = net
        self.b["spaces"].append(m)
        return u

    def ayrim(self, a, b, duvar="EI 120", acik=0.0, pencere=None, bosluk=False, kapi=None):
        """İki mahal arasında duvar (``duvar`` beyanıyla; yok: beyansız), açık boy, pencere, boşluk ve kapı."""
        d = {"uid": uid(), "kind": "wall", "length": 4.0}
        if duvar:
            d["fireRating"] = duvar
        acikliklar = []
        if pencere is not None:
            p = {"uid": uid(), "kind": "window", "length": 1.0}
            if pencere:
                p["fireRating"] = pencere
            acikliklar.append(p)
        if bosluk:
            acikliklar.append({"uid": uid(), "kind": "gap", "length": 1.0})
        if kapi is not None:
            k = {"uid": uid(), "kind": "door", "length": 1.0}
            if kapi:
                k["fireRating"] = kapi
            acikliklar.append(k)
            self.kapi(a, b, kapi)
        self.b["separations"].append({"level": "b01", "rooms": [a, b], "open": acik, "walls": [d],
                                      "openings": acikliklar})

    def kapi(self, a, b, sinif, merdiven=False):
        k = {"uid": uid(), "level": "b01", "between": [a, b], "operation": "swing", "width": 1.0, "clear": 0.9}
        if sinif:
            k["fireRating"] = sinif
        if merdiven:
            k["stair"] = True
        self.b["doors"].append(k)


def kural(a=None, b=None, c=None, d=None, s_b="9.9.2009"):
    return {"BYKHY 54/2": [a, "19.12.2007"], "BYKHY 54/4": [b, s_b], "BYKHY 54/5": [c, "19.12.2007"],
            "BYKHY 56/1": [d, "19.12.2007"]}


def bulgu(code, has, state=None):
    out = {"code": code, "has": has}
    if state:
        out["state"] = state
    return out


def durum(id_, neden, bina, bulgular, kurallar, izin=TARIH):
    return {"id": id_, "neden": neden, "permit": izin, "building": bina.b,
            "expect": {"findings": bulgular, "rules": kurallar}}


def kazanli(net=40.0, duvar="EI 120", kapi=KAPI_IYI, ad="Kazan dairesi"):
    """Koridora açılan tek kapılı kazan dairesi."""
    b = Bina()
    k = b.mahal(ad, net)
    b.ayrim(k, b.koridor, duvar=duvar, kapi=kapi)
    return b, k


def durumlar():
    out = []
    b, _ = kazanli()
    out.append(durum("iyi", "40 m² kazan dairesi, duvarı EI 120, koridora EI2 90-S200C5 tek kapı: 54/2 ve 54/4 geçer; "
                     "54/5 ısıl kapasite çizimde yok, tek kapı → uzman incelemesi; yakıt deposu yok.", b,
                     [bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("passed", "passed", "expertReview", "outOfScope")))
    b = Bina()
    b.mahal("Depo", 10.0)
    out.append(durum("kazansiz", "Kazan dairesi yok → hepsi kapsam dışı.", b, [],
                     kural("outOfScope", "outOfScope", "outOfScope", "outOfScope")))
    b, _ = kazanli(duvar="EI 60")
    out.append(durum("duvar-60", "Kazan dairesi ile koridor arası EI 60: 54/2 'en az 120 dakika' → kalır.", b,
                     [bulgu(AYRIM, ["EI 60", "en az EI 120"]), bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("failed", "passed", "expertReview", "outOfScope")))
    b, _ = kazanli(duvar=None)
    out.append(durum("duvar-beyansiz", "Duvarın sınıfı beyan edilmemiş → 54/2 eksik veri.", b,
                     [bulgu(AYRIM, ["beyan edilmemiş"], "missingData"), bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("missingData", "passed", "expertReview", "outOfScope")))
    b, _ = kazanli(duvar="REI 120")
    out.append(durum("rei-120", "Taşıyıcı duvar REI 120: bütünlük ve yalıtım 120 dakika → 54/2 geçer.", b,
                     [bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("passed", "passed", "expertReview", "outOfScope")))
    b, _ = kazanli(duvar="E 120")
    out.append(durum("e-120", "Duvar yalnız E 120 (yalıtımsız): bölme yangına dayanıklı ayrım değil → 54/2 kalır.", b,
                     [bulgu(AYRIM, ["E 120"]), bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("failed", "passed", "expertReview", "outOfScope")))
    b = Bina()
    k = b.mahal("Kazan dairesi", 40.0)
    b.ayrim(k, b.koridor, kapi=KAPI_IYI)
    d = b.mahal("Depo", 15.0)
    b.ayrim(k, d, acik=1.5)
    out.append(durum("acik-ayrim", "Kazan dairesi ile depo arasında 1,50 m duvarsız yer → 54/2 kalır.", b,
                     [bulgu(AYRIM, ["duvarsız 1,50 m"]), bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("failed", "passed", "expertReview", "outOfScope")))
    b = Bina()
    k = b.mahal("Kazan dairesi", 40.0)
    b.ayrim(k, b.koridor, kapi=KAPI_IYI, pencere="E 30")
    out.append(durum("pencere", "Koridora bakan iç pencere E 30: bölmenin parçası, 120 dakika değil → 54/2 kalır.", b,
                     [bulgu(AYRIM, ["pencere", "E 30"]), bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("failed", "passed", "expertReview", "outOfScope")))
    b = Bina()
    k = b.mahal("Kazan dairesi", 40.0)
    b.ayrim(k, b.koridor, kapi=KAPI_IYI, bosluk=True)
    out.append(durum("bosluk", "Koridora eşiksiz boşluk: ayrım yok → 54/2 kalır.", b,
                     [bulgu(AYRIM, ["eşiksiz boşluk"]), bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("failed", "passed", "expertReview", "outOfScope")))
    b = Bina()
    k = b.mahal("Kazan dairesi", 40.0)
    m = b.mahal("Merdiven", 18.0, merdiven=True)
    b.ayrim(k, m, kapi=None)
    b.kapi(k, m, KAPI_IYI, merdiven=True)
    out.append(durum("merdivene", "Kapısı merdivene açılıyor: 54/4 'kaçış merdivenine ... doğrudan açılmaması' → kalır.",
                     b, [bulgu(KAPI, ["merdivene açılıyor"]), bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("passed", "failed", "expertReview", "outOfScope")))
    b = Bina()
    k = b.mahal("Kazan dairesi", 40.0)
    o = b.mahal("Otopark", 300.0)
    b.ayrim(k, o, kapi=KAPI_IYI)
    out.append(durum("otoparka", "Kapısı otoparka açılıyor: 'mutlaka bir ortak hol veya koridora' → kalır.", b,
                     [bulgu(KAPI, ["“Otopark”", "hol ya da koridora"]), bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("passed", "failed", "expertReview", "outOfScope")))
    b = Bina()
    k = b.mahal("Kazan dairesi", 40.0)
    b.kapi(k, None, KAPI_IYI)
    out.append(durum("disari", "Tek kapısı dışarı açılıyor: fıkra hol ya da koridor der, dışarıyı anmaz → uzman "
                     "incelemesi.", b, [bulgu(KAPI, ["dışarı"], "expertReview"), bulgu(CIKIS, ["350 kW"],
                                                                                         "expertReview")],
                     kural("outOfScope", "expertReview", "expertReview", "outOfScope")))
    b = Bina()
    b.mahal("Kazan dairesi", 40.0)
    out.append(durum("kapisiz", "Kazan dairesinin kapısı çizilmemiş → 54/4 ve 54/5 eksik veri.", b,
                     [bulgu(KAPI, ["kapısı çizilmemiş"], "missingData"),
                      bulgu(CIKIS, ["kapısı çizilmemiş"], "missingData")],
                     kural("outOfScope", "missingData", "missingData", "outOfScope")))
    b, k = kazanli(net=120.0)
    out.append(durum("buyuk-tek-kapi", "120 m² kazan dairesi tek kapı: 54/5 '100 m2'nin üzerindeki ... en az 2 çıkış "
                     "kapısı' → kalır.", b, [bulgu(CIKIS, ["120,00 m² > 100 m²", "1 kapı"])],
                     kural("passed", "passed", "failed", "outOfScope")))
    b, k = kazanli(net=120.0)
    o = b.mahal("Koridor", 15.0)
    b.ayrim(k, o, kapi=KAPI_IYI)
    out.append(durum("buyuk-iki-kapi", "120 m², iki koridora iki kapı: 54/5 geçer (yönleri idarenin).", b, [],
                     kural("passed", "passed", "passed", "outOfScope")))
    b, k = kazanli(net=100.0)
    out.append(durum("tam-100", "Tam 100 m², tek kapı: '100 m2'nin üzerindeki' değil; kapasite çizimde yok → uzman "
                     "incelemesi.", b, [bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("passed", "passed", "expertReview", "outOfScope")))
    b, k = kazanli(kapi="EI2 60-S200C5")
    out.append(durum("kapi-60", "Kapı EI2 60-S200C5: 54/5 'en az 90 dakika' → kalır.", b,
                     [bulgu(CIKIS, ["350 kW"], "expertReview"), bulgu(SINIF, ["60 dakika < 90 dakika"])],
                     kural("passed", "passed", "failed", "outOfScope")))
    b, k = kazanli(kapi="EI2 90")
    out.append(durum("kapi-s-c-yok", "Kapı EI2 90: duman sızdırmaz ve kendiliğinden kapanır değil → kalır.", b,
                     [bulgu(CIKIS, ["350 kW"], "expertReview"), bulgu(SINIF, ["S yok", "C yok"])],
                     kural("passed", "passed", "failed", "outOfScope")))
    b, k = kazanli(kapi="")
    out.append(durum("kapi-beyansiz", "Kapının sınıfı beyan edilmemiş → 54/5 eksik veri.", b,
                     [bulgu(CIKIS, ["350 kW"], "expertReview"), bulgu(SINIF, ["beyan edilmemiş"], "missingData")],
                     kural("passed", "passed", "missingData", "outOfScope")))
    b, k = kazanli(ad="Kalorifer dairesi")
    out.append(durum("kalorifer", "Adı Kalorifer dairesi: kazan dairesi → aynı denetim.", b,
                     [bulgu(CIKIS, ["Kalorifer dairesi"], "expertReview")],
                     kural("passed", "passed", "expertReview", "outOfScope")))
    b, k = kazanli(ad="Isı merkezi")
    out.append(durum("isi-merkezi", "Adı Isı merkezi: kazan dairesi → aynı denetim.", b,
                     [bulgu(CIKIS, ["Isı merkezi"], "expertReview")],
                     kural("passed", "passed", "expertReview", "outOfScope")))
    b, k = kazanli()
    y_ = b.mahal("Yakıt deposu", 12.0)
    b.ayrim(y_, k, duvar="EI 120", kapi="EI2 120-S200C5")
    out.append(durum("yakit-iyi", "Yakıt deposu kazan dairesinden EI 120 duvar ve EI2 120 kapıyla ayrılmış → 56/1 geçer; "
                     "depoyla ayrım 54/2'nin değil, depoya açılan kapı kazan dairesinin çıkış kapısı değil (54/4, 54/5 "
                     "yalnız koridor kapısını okur: tek çıkış, kapasite uzman incelemesi).", b,
                     [bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("passed", "passed", "expertReview", "passed")))
    b, k = kazanli()
    y_ = b.mahal("Akaryakıt deposu", 12.0)
    b.ayrim(y_, k, duvar="EI 90", kapi="EI2 120-S200C5")
    out.append(durum("yakit-90", "Akaryakıt deposu ile kazan dairesi arası EI 90: 56/1 '120 dakika' → kalır.", b,
                     [bulgu(CIKIS, ["350 kW"], "expertReview"), bulgu(YAKIT, ["EI 90", "en az EI 120"])],
                     kural("passed", "passed", "expertReview", "failed")))
    b, k = kazanli()
    y_ = b.mahal("Yakıt deposu", 12.0)
    b.ayrim(y_, k, duvar="EI 120", kapi="EI2 90-S200C5")
    out.append(durum("yakit-kapi-90", "Yakıt deposunun kazan dairesine kapısı EI2 90: bölmenin parçası, 120 dakika değil "
                     "→ 56/1 kalır.", b, [bulgu(CIKIS, ["350 kW"], "expertReview"), bulgu(YAKIT, ["kapı", "EI2 90-S200C5"])],
                     kural("passed", "passed", "expertReview", "failed")))
    b, _ = kazanli()
    out.append(durum("basvuru-2008", "Başvuru 1.6.2008: 54/4'ün 9.9.2009 öncesi metni kural setinde yok → satırı uzman "
                     "incelemesi; öbürleri 2007'den.", b, [bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("passed", "expertReview", "expertReview", "outOfScope", s_b=None),
                     izin={"particulars": {"basvuru_tarihi": "01.06.2008"}}))
    b, _ = kazanli(duvar="EI 60")
    out.append(durum("tarihsiz", "Başvuru tarihi yok: sonuç günle değişmiyor → en yeni hâl, 54/2 kalır.", b,
                     [bulgu(AYRIM, ["EI 60"]), bulgu(CIKIS, ["350 kW"], "expertReview")],
                     kural("failed", "passed", "expertReview", "outOfScope"), izin=None))
    return out


def belge():
    return {
        "format": "kentos.regulationCases", "version": 1,
        "about": ("Kazan dairesi ve yakıt deposunun bağımsız durumları (docs/adr/8113): beklenen her sonuç Binaların "
                  "Yangından Korunması Hakkında Yönetmelik 54/2, 54/5, 56/1'in (RG 19.12.2007/26735) ve 54/4'ün (RG "
                  "9.9.2009/27344) metninden elle çıkarıldı; kuralın kodundan üretilmedi (scripts/fixtures/kazan_cases.py "
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
