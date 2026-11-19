#!/usr/bin/env python3
"""Otel yatak odasının içindeki kaçış uzaklığının bağımsız durumları (docs/adr/8111): BYKHY 50/3.

Her durumun beklenen sonucu yönetmeliğin metninden (mevzuat.gov.tr derlenmiş metni, SHA-256 d82d06d2…a7d9) elle
yazıldı; betik yalnız olguları kurar ve beklenenleri olduğu gibi yazar, kuralın kodunu çağırmaz. Yazar:
fixtures/standards/v1/konaklama.json; `--check` karşılaştırır, yazmaz.

    python3 scripts/fixtures/konaklama_cases.py [--check]
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/standards/v1/konaklama.json"
TARIH = {"particulars": {"basvuru_tarihi": "02.07.2026"}}
KOD = "bykhy_hotel_room_distance"
_n = [0]


def uid():
    _n[0] += 1
    return f"00000000-0000-4000-8000-{_n[0]:012d}"


class Bina:
    """Bir binanın olguları: katlar, mahaller (adı, katı, en uzak noktasından kapısına yürüyüş), kapılar (iki yanı),
    yağmurlama beyanı."""

    def __init__(self, uses, yagmurlama=False, bolum=0, konut=0):
        self.b = {"storeys": [{"level": f"k{i:02d}", "name": "Zemin kat" if i == 0 else f"{i}. kat",
                               "elevation": 3.0 * i, "units": 0, "common": 1, "annexes": 0, "other": 0, "technical": 0,
                               "inhabited": 0, "elevators": 0, "stair": True} for i in range(3)],
                  "units": bolum, "dwellings": konut, "uses": uses, "spaces": [], "doors": [], "structureHeight": 9.0}
        if yagmurlama:
            self.b["sprinklers"] = True
        self.koridor = self.mahal("Koridor", None)

    def mahal(self, ad, uzak, kapi=0):
        """Bir mahal; en uzak noktasından kapısına ``uzak`` m (yok: kapısı çizilmemiş) ve koridora ``kapi`` kapısı."""
        u = uid()
        m = {"uid": u, "level": "k01", "name": ad, "area": 30.0, "stair": False}
        if uzak is not None:
            m["toDoor"] = uzak
        self.b["spaces"].append(m)
        for _ in range(kapi):
            self.b["doors"].append({"uid": uid(), "level": "k01", "between": [u, self.koridor], "width": 0.9,
                                    "clear": 0.85})
        return u


def kural(durum, surum="19.12.2007"):
    return {"BYKHY 50/3": [durum, surum]}


def bulgu(has, state=None):
    out = {"code": KOD, "has": has}
    if state:
        out["state"] = state
    return out


def durum(id_, neden, bina, bulgular, kurallar, izin=TARIH):
    return {"id": id_, "neden": neden, "permit": izin, "building": bina.b,
            "expect": {"findings": bulgular, "rules": kurallar}}


def otel(uzak, kapi=1, ad="Oda 101", yagmurlama=False, uses=("Otel",)):
    b = Bina(list(uses), yagmurlama)
    b.mahal(ad, uzak, kapi)
    return b


def durumlar():
    out = []
    out.append(durum("kisa", "Otel odası, en uzak noktadan kapısına 8 m: 15 m'yi aşmıyor, tek kapı yeter → geçer.",
                     otel(8.0), [], kural("passed")))
    out.append(durum("uzun-tek-kapi", "17 m, tek kapı: 50/3-a 'aşması hâlinde ... en az 2 çıkış kapısı' → kalır.",
                     otel(17.0), [bulgu(["17,00 m > 15,00 m", "1 kapı", "BYKHY 50/3-a"])], kural("failed")))
    out.append(durum("uzun-iki-kapi", "17 m, iki kapı: (a) iki kapı ister, var; birbirinden uzaklığı ölçü değil → "
                     "geçer.", otel(17.0, kapi=2), [], kural("passed")))
    out.append(durum("yagmurlamali-17", "Tamamı yağmurlamalı otel, 17 m tek kapı: (b) 20 m'yi aşmıyor → geçer.",
                     otel(17.0, yagmurlama=True), [], kural("passed")))
    out.append(durum("yagmurlamali-22", "Yağmurlamalı, 22 m tek kapı: (b) '20 m'yi aşmaması gerekir' → kalır.",
                     otel(22.0, yagmurlama=True), [bulgu(["22,00 m > 20,00 m", "BYKHY 50/3-b"])], kural("failed")))
    out.append(durum("yagmurlamali-22-iki-kapi", "Yağmurlamalı, 22 m iki kapı: (b) aşmamasını, (a) iki kapıyı söyler, "
                     "birlikte okunuşu idarenin → uzman incelemesi.", otel(22.0, kapi=2, yagmurlama=True),
                     [bulgu(["22,00 m > 20,00 m", "2 kapı"], "expertReview")], kural("expertReview")))
    out.append(durum("tam-15", "Tam 15,00 m: 'aşmaması hâlinde' → geçer.", otel(15.0), [], kural("passed")))
    out.append(durum("15-01", "15,01 m tek kapı → kalır.", otel(15.01), [bulgu(["15,01 m"])], kural("failed")))
    out.append(durum("15-005", "15,005 m tek kapı → kalır; ileti milimetreyi gösterir.", otel(15.005),
                     [bulgu(["15,005 m"])], kural("failed")))
    out.append(durum("yagmurlamali-tam-20", "Yağmurlamalı, tam 20,00 m → geçer.", otel(20.0, yagmurlama=True), [],
                     kural("passed")))
    out.append(durum("kapisiz", "Odanın kapısı çizilmemiş: uzaklık ölçülemedi → eksik veri.", otel(None, kapi=0),
                     [bulgu(["kapısı çizilmemiş"], "missingData")], kural("missingData")))
    out.append(durum("suit", "Süit, 16 m tek kapı: 'süit odada' → kalır.", otel(16.0, ad="Süit 301"),
                     [bulgu(["Süit 301", "16,00 m"])], kural("failed")))
    out.append(durum("otel-odasi-adi", "Adı “Otel odası 12”, 16 m tek kapı → kalır.", otel(16.0, ad="Otel odası 12"),
                     [bulgu(["Otel odası 12"])], kural("failed")))
    out.append(durum("motel", "Motel, 16 m tek kapı → kalır.", otel(16.0, uses=("Motel",)), [bulgu(["16,00 m"])],
                     kural("failed")))
    out.append(durum("pansiyon", "Pansiyon, 16 m tek kapı → kalır.", otel(16.0, uses=("Pansiyon",)),
                     [bulgu(["16,00 m"])], kural("failed")))
    out.append(durum("yurt", "Öğrenci yurdu, 16 m tek kapı: 50/3 otel yatak odasını söyler → kapsam dışı.",
                     otel(16.0, uses=("Öğrenci yurdu",)), [], kural("outOfScope")))
    b = Bina(["Konut"], bolum=4, konut=4)
    b.mahal("Yatak odası", 16.0, 1)
    out.append(durum("konut", "Konut: otel değil → kapsam dışı.", b, [], kural("outOfScope")))
    b = Bina(["Büro"])
    b.mahal("Oda 4", 16.0, 1)
    out.append(durum("buro", "Büro: otel değil → kapsam dışı.", b, [], kural("outOfScope")))
    b = Bina(["Otel"])
    b.mahal("Lobi", 25.0, 1)
    b.mahal("Toplantı odası", 18.0, 1)
    out.append(durum("odasiz-otel", "Otelin bu katında yatak odası yok (lobi, toplantı odası) → kapsam dışı.", b, [],
                     kural("outOfScope")))
    b = Bina(["Otel"])
    b.mahal("Oda 101", 9.0, 1)
    b.mahal("Oda 102", 18.0, 1)
    out.append(durum("iki-oda", "Biri 9 m, biri 18 m tek kapı → yalnız Oda 102 kalır.", b,
                     [bulgu(["Oda 102", "18,00 m"])], kural("failed")))
    b = Bina(["Otel"])
    b.mahal("Süit 301", 9.0, 1)
    b.mahal("Süit 301 banyosu", 16.0, 1)
    out.append(durum("suit-banyosu", "Süitin banyosu otel odası değil (9 m'lik süit geçer) → geçer.", b, [],
                     kural("passed")))
    out.append(durum("karma", "Otel ve büro karma, 16 m tek kapı → kalır.", otel(16.0, uses=("Otel", "Büro")),
                     [bulgu(["16,00 m"])], kural("failed")))
    out.append(durum("tarihsiz", "Başvuru tarihi yok: 50/3 2007'den beri aynı → en yeni hâl, kalır.", otel(17.0),
                     [bulgu(["17,00 m"])], kural("failed"), izin=None))
    out.append(durum("basvuru-2007", "Başvuru 1.6.2007: yönetmelik 19.12.2007'de yürürlüğe girdi, öncesi kural setinde "
                     "yok → satır uzman incelemesi.", otel(17.0), [], kural("expertReview", None),
                     izin={"particulars": {"basvuru_tarihi": "01.06.2007"}}))
    b = Bina([])
    b.mahal("Otel odası 5", 16.0, 1)
    out.append(durum("adlardan", "Bina programında kullanım yok, mahal adı Otel odası: konaklama mahal adından → kalır.",
                     b, [bulgu(["16,00 m"])], kural("failed")))
    out.append(durum("otel-ve-yurt", "Kullanımı “Otel ve yurt”: otel sözcüğü geçen bina oteldir (docs/adr/8156), 16 m "
                     "tek kapı → kalır.", otel(16.0, uses=("Otel ve yurt",)), [bulgu(["16,00 m"])], kural("failed")))
    return out


def belge():
    return {
        "format": "kentos.regulationCases", "version": 1,
        "about": ("Otel yatak odasının içindeki kaçış uzaklığının bağımsız durumları (docs/adr/8111): beklenen her sonuç "
                  "Binaların Yangından Korunması Hakkında Yönetmelik 50/3'ün metninden (RG 19.12.2007/26735) elle "
                  "çıkarıldı; kuralın kodundan üretilmedi (scripts/fixtures/konaklama_cases.py olguları kurar, "
                  "beklenenleri olduğu gibi yazar)."),
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
