#!/usr/bin/env python3
"""The roofs' fire rule's independent cases (docs/adr/8134): fixtures/standards/v1/catilar.json.

Every expected result is stated here by hand from Binaların Yangından Korunması Hakkında Yönetmelik 4/ccc, 28/2, 28/3 and
Ek-2/Ç (the compiled official text), never computed by the rule's code:

- 28/2: a roof covering's material is BROOF (TS EN 13501-5, any test method or none); the surface under the covering
  (kaplama tahtası) and the insulations (su and ısı yalıtımı) are at least zor alevlenici; under a covering of hiç yanmaz
  material (A1) the surface may be normal alevlenici. A2-s1,d0 (zor yanıcı) is not said to be yanmaz: a result resting
  on it is expert review. Battens and rafters are not read by 28/2. A layer without a material or a declared class is
  missing data; a roof with no covering layer is missing data.
- 28/3-b: in a high building (building height over 21.50 m or structure height over 30.50 m) the roof's structure
  (rafters) and covering are yanmaz: hiç yanmaz holds, zor yanıcı is expert review, anything else fails, unknown is
  missing data. In a building that is not high the article holds if it stands in attached order, which the drawing does
  not say: a roof that would fail is expert review, others out of scope.
- An application before 9.9.2009: the article's text is not in the rule set, expert review.

    python3 scripts/fixtures/roof_fire_cases.py            # writes the cases
    python3 scripts/fixtures/roof_fire_cases.py --check    # writes nothing; compares
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/standards/v1/catilar.json"

PASSED, FAILED, MISSING, EXPERT, OUT_OF = "passed", "failed", "missingData", "expertReview", "outOfScope"
V = "9.9.2009"


def uid(n):
    return f"00000000-0000-4000-8000-{n:012d}"


def layer(kind, material=None, reaction=None, roof_class=None):
    out = {"kind": kind}
    if material:
        out["material"] = material
    if reaction:
        out["reaction"] = reaction
    if roof_class:
        out["roofClass"] = roof_class
    return out


TILE = layer("covering", "Kil kiremit", "A1", "BROOF(t1)")
BATTENS = layer("battens", "Çam lata")
OSB_B = layer("boarding", "OSB", "B-s1,d0")
PINE_D = layer("boarding", "Çam tahta", "D-s2,d0")
STONE_WOOL = layer("insulation", "Taş yünü", "A1")
EPS_E = layer("insulation", "EPS", "E")
STEEL = layer("rafters", "Çelik", "A1")
PINE_RAFTERS = layer("rafters", "Çam mertek", "D-s2,d0")


#: What a storey holds as the other rules read it: nothing (a storey's units and rooms are not this rule's).
EMPTY = {"units": 0, "common": 0, "annexes": 0, "other": 0, "technical": 0, "inhabited": 0, "elevators": 0}


def building(roofs, height=6.0, structure=8.5):
    return {
        "storeys": [dict(level=level, name=name, elevation=z, **EMPTY) for level, name, z in
                    (("k00", "Zemin kat", 0.0), ("k01", "1. kat", 3.0))],
        "height": height,
        "structureHeight": structure,
        "roofs": [{"uid": uid(i + 1), "level": "k01", "layers": layers} for i, layers in enumerate(roofs)],
    }


def case(id_, why, roofs, findings, r2, r3, height=6.0, structure=8.5, day="02.07.2026"):
    version = V
    return {
        "id": id_,
        "neden": why,
        "permit": {"particulars": {"basvuru_tarihi": day}},
        "building": building(roofs, height, structure),
        "expect": {"findings": findings, "rules": {"BYKHY 28/2": [r2, version], "BYKHY 28/3": [r3, version]}},
    }


def f(code, state=None, has=()):
    out = {"code": code}
    if state:
        out["state"] = state
    if has:
        out["has"] = list(has)
    return out


COVER, UNDER, NONC = "bykhy_roof_covering_class", "bykhy_roof_underlay_reaction", "bykhy_roof_noncombustible"


def cases():
    out = [
        case("tam", "Kil kiremit A1 BROOF(t1), OSB B (zor alevlenici), taş yünü A1, çelik mertek; 6 m'lik bina → 28/2 geçer, "
             "28/3 yüksek olmayan binada hepsi hiç yanmaz: kapsam dışı.",
             [[TILE, BATTENS, OSB_B, STONE_WOOL, STEEL]], [], PASSED, OUT_OF),
        case("kaplama-croof", "Kaplama CROOF(t3): 28/2 BROOF ister → kalır.",
             [[layer("covering", "Kompozit levha", "A1", "CROOF(t3)"), OSB_B, STEEL]],
             [f(COVER, has=["CROOF(t3)", "BROOF"])], FAILED, OUT_OF),
        case("kaplama-sinifsiz", "Kaplamanın dış yangın sınıfı beyan edilmemiş → eksik veri.",
             [[layer("covering", "Kil kiremit", "A1"), OSB_B, STEEL]], [f(COVER, MISSING, ["dış yangın sınıfı"])], MISSING, OUT_OF),
        case("kaplama-malzemesiz", "Kaplamanın malzemesi yok → eksik veri.",
             [[layer("covering"), OSB_B, STEEL]], [f(COVER, MISSING, ["malzemesi yok"])], MISSING, OUT_OF),
        case("katmansiz", "Katmansız çatı tek kaplamadır (Kiremit), sınıfları beyansız → 28/2 eksik veri; 28/3 yüksek "
             "olmayan binada bilinmeyen sınıf söylenmez.",
             [[layer("covering", "Kiremit")]], [f(COVER, MISSING)], MISSING, OUT_OF),
        case("broof-yontemsiz", "Ek-2/Ç'nin yazdığı gibi yalnız BROOF → geçer.",
             [[layer("covering", "Arduvaz", "A1", "BROOF"), OSB_B, STEEL]], [], PASSED, OUT_OF),
        case("yanmaz-kaplama-normal-tahta", "Hiç yanmaz (A1) kaplamanın üzerine uygulandığı tahta D (normal alevlenici) → "
             "28/2'nin istisnası: geçer; ahşap mertek yüksek olmayan binada uzman incelemesi (bitişik nizamda kalır).",
             [[TILE, BATTENS, PINE_D, PINE_RAFTERS]], [f(NONC, EXPERT, ["nizam"])], PASSED, EXPERT),
        case("yanici-kaplama-normal-tahta", "Bitümlü şingıl E (normal alevlenici) BROOF(t1), tahta D → kaplama yanmaz değil, "
             "tahta en az zor alevlenici olmalı: kalır; kaplama yanmaz değil: 28/3 uzman incelemesi.",
             [[layer("covering", "Bitümlü şingıl", "E", "BROOF(t1)"), PINE_D]],
             [f(UNDER, has=["kaplama tahtası", "zor alevlenici"]), f(NONC, EXPERT)], FAILED, EXPERT),
        case("zor-yanici-kaplama-normal-tahta", "Lifli çimento levha A2-s1,d0 (zor yanıcı) BROOF, tahta D: yanmaz sayılıp "
             "sayılmadığı yazılı değil → 28/2 uzman incelemesi; 28/3 de uzman incelemesi.",
             [[layer("covering", "Lifli çimento levha", "A2-s1,d0", "BROOF(t2)"), PINE_D]],
             [f(UNDER, EXPERT, ["zor yanıcı"]), f(NONC, EXPERT)], EXPERT, EXPERT),
        case("yanmaz-kaplama-eps", "Hiç yanmaz kaplamanın altında EPS E: istisna yalnız yüzeyindir, yalıtım en az zor "
             "alevlenici → kalır.",
             [[TILE, OSB_B, EPS_E, STEEL]], [f(UNDER, has=["ısı yalıtımı", "EPS"])], FAILED, OUT_OF),
        case("su-yalitimi-sinifsiz", "Su yalıtımının yanıcılık sınıfı beyan edilmemiş → eksik veri.",
             [[TILE, layer("waterproofing", "Bitümlü örtü"), OSB_B, STEEL]], [f(UNDER, MISSING, ["su yalıtımı"])], MISSING, OUT_OF),
        case("su-yalitimi-malzemesiz", "Su yalıtımının malzemesi yok → eksik veri.",
             [[TILE, layer("waterproofing"), OSB_B, STEEL]], [f(UNDER, MISSING, ["malzemesi yok"])], MISSING, OUT_OF),
        case("kaplama-sinifi-okunmayan-tahta-normal", "Kaplamanın yanıcılık sınıfı beyansız, tahta D: kaplama yanmazsa yeter, "
             "değilse yetmez → eksik veri.",
             [[layer("covering", "Kil kiremit", None, "BROOF(t1)"), PINE_D, STEEL]], [f(UNDER, MISSING)], MISSING, OUT_OF),
        case("lata-ve-mertek-28-2-okunmaz", "Lata (malzemesiz) ve ahşap mertek 28/2'nin katmanları değil → 28/2 geçer; "
             "mertek yüksek olmayan binada uzman incelemesi.",
             [[TILE, layer("battens"), OSB_B, PINE_RAFTERS]], [f(NONC, EXPERT)], PASSED, EXPERT),
        case("kaplama-katmani-yok", "Katmanlarında kaplama yok; tahta D → kaplama eksik veri, tahtanın isteği bilinmiyor: "
             "eksik veri.",
             [[PINE_D, STEEL]], [f(COVER, MISSING, ["kaplama katmanı yok"]), f(UNDER, MISSING)], MISSING, OUT_OF),
        case("yuksek-yanmaz", "24 m'lik yüksek bina; kaplama ve mertek hiç yanmaz, tahta A2-s1,d0 → iki madde de geçer.",
             [[TILE, layer("boarding", "Çimento levha", "A2-s1,d0"), STEEL]], [], PASSED, PASSED, height=24.0, structure=27.0),
        case("yuksek-ahsap-mertek", "Yüksek binada çam mertek D → 28/3 kalır.",
             [[TILE, OSB_B, PINE_RAFTERS]], [f(NONC, has=["yüksek binada", "mertek"])], PASSED, FAILED, height=24.0,
             structure=27.0),
        case("yuksek-mertek-sinifsiz", "Yüksek binada mertek sınıfsız → 28/3 eksik veri.",
             [[TILE, OSB_B, layer("rafters", "Ahşap mertek")]], [f(NONC, MISSING)], PASSED, MISSING, height=24.0,
             structure=27.0),
        case("yuksek-zor-yanici-kaplama", "Yüksek binada kaplama A2-s1,d0 BROOF, çelik mertek → 28/3 uzman incelemesi.",
             [[layer("covering", "Lifli çimento levha", "A2-s1,d0", "BROOF(t4)"), OSB_B, STEEL]], [f(NONC, EXPERT)],
             PASSED, EXPERT, height=24.0, structure=27.0),
        case("yapi-yuksekligi-yuksek", "Bina yüksekliği 18 m ama yapı yüksekliği 31 m: yüksek bina (4/ccc) → çam mertek "
             "kalır.",
             [[TILE, OSB_B, PINE_RAFTERS]], [f(NONC)], PASSED, FAILED, height=18.0, structure=31.0),
        case("iki-cati", "İki çatıdan biri CROOF(t4) → bir bulgu.",
             [[TILE, OSB_B, STEEL], [layer("covering", "Metal levha", "A1", "CROOF(t4)"), OSB_B, STEEL]],
             [f(COVER, has=["CROOF(t4)"])], FAILED, OUT_OF),
        case("cati-yok", "Çatısı olmayan bina → iki madde de kapsam dışı.", [], [], OUT_OF, OUT_OF),
    ]
    # An application before 9.9.2009: the articles' text is not in the rule set.
    old = case("eski-basvuru", "Başvuru 1.6.2009: 28/2 ve 28/3'ün kural setindeki ilk hâli (9.9.2009) sonra → iki madde "
               "de uzman incelemesi.", [[TILE, OSB_B, STEEL]],
               [f("regulation_text_unknown", EXPERT, ["9.9.2009"]), f("regulation_text_unknown", EXPERT, ["9.9.2009"])],
               EXPERT, EXPERT, day="01.06.2009")
    old["expect"]["rules"] = {"BYKHY 28/2": [EXPERT, None], "BYKHY 28/3": [EXPERT, None]}
    out.append(old)
    return out


def document():
    return {
        "format": "kentos.regulationCases",
        "version": 1,
        "about": ("Çatıların yanıcılığının bağımsız durumları (docs/adr/8134): beklenen her sonuç Binaların Yangından "
                  "Korunması Hakkında Yönetmelik 4/ccc, 28/2, 28/3 ve Ek-2/Ç'nin derlenmiş resmî metninden elle "
                  "çıkarıldı (scripts/fixtures/roof_fire_cases.py); kuralın kodundan üretilmedi. Olgular KentOS'un "
                  "ölçüsüdür (building.roofs)."),
        "cases": cases(),
    }


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.is_file() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırıp farkı okuyun.")
        print(f"{OUT.relative_to(ROOT)}: {len(document()['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(document()['cases'])} durum yazıldı.")


if __name__ == "__main__":
    main()
