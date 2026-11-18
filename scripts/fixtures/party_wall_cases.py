#!/usr/bin/env python3
"""The party wall rule's independent cases (docs/adr/8142): fixtures/standards/v1/bitisik.json.

Every expected result is stated here by hand from Planlı Alanlar İmar Yönetmeliği 39/4 and Binaların Yangından
Korunması Hakkında Yönetmelik 25/1, 25/2 (the compiled official texts), never computed by the rule's code:

- 39/4 (in force 1.10.2017): in bitişik and blok nizam no window and no door in the attached wall on the neighbouring
  parcel's boundary. A party edge says the building is attached there (bitişik, or blok/ikiz): every window and door
  standing on it fails, whatever the layout written; a gap (neither) is not a window or a door.
- 25/1 (19.12.2007): the fire wall between attached buildings resists at least 90 minutes: a wall on a party edge is at
  least EI 90, REI 90 when it is declared load-bearing; no class declared is missing data, less fails.
- 25/2: no hole and no gap in a fire wall; a door or a fixed light that cannot be avoided resists at least half the
  wall's time (E 45), a door self-closing and smoke-tight. A gap fails; a door or a window under E 45 fails, without a
  class is missing data; a door of E 45 or more without C or S in its class is missing data; a door with both and a
  window of E 45 or more are expert review (the unavoidability; the window's being fixed).
- An application before an article's first text in the rule set: expert review.

    python3 scripts/fixtures/party_wall_cases.py            # writes the cases
    python3 scripts/fixtures/party_wall_cases.py --check    # writes nothing; compares
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/standards/v1/bitisik.json"

PASSED, FAILED, MISSING, EXPERT, OUT_OF = "passed", "failed", "missingData", "expertReview", "outOfScope"
V39, V25 = "1.10.2017", "19.12.2007"

#: What a storey holds as the other rules read it: nothing.
EMPTY = {"units": 0, "common": 0, "annexes": 0, "other": 0, "technical": 0, "inhabited": 0, "elevators": 0}


def uid(n):
    return f"00000000-0000-4000-8000-{n:012d}"


def wall(n, rating=None, length=9.4):
    out = {"uid": uid(n), "kind": "wall", "length": length}
    if rating:
        out["fireRating"] = rating
    return out


def opening(n, kind, rating=None, length=1.0):
    out = {"uid": uid(n), "kind": kind, "length": length}
    if rating:
        out["fireRating"] = rating
    return out


def building(party, bearing=()):
    out = {
        "storeys": [dict(level=level, name=name, elevation=z, **EMPTY) for level, name, z in
                    (("k00", "Zemin kat", 0.0), ("k01", "1. kat", 3.0))],
        "height": 6.0,
        "structureHeight": 6.5,
        "layout": "attached",
    }
    if party:
        out["party"] = party
    walls = sorted({w["uid"] for p in party for w in p.get("walls", [])})
    if walls:
        out["elements"] = [dict(uid=u, kind="wall", level=None, frame=False, exterior=True, escape=False,
                                fireStop=False, **({"loadBearing": True} if u in bearing else {})) for u in walls]
    return out


def side(level, walls, openings=(), open_=0.0, edge=2):
    out = {"edge": edge, "level": level, "walls": list(walls), "open": open_}
    if openings:
        out["openings"] = list(openings)
    return out


def case(id_, why, party, findings, r39, r251, r252, day="02.07.2026", bearing=(), versions=(V39, V25, V25)):
    return {
        "id": id_,
        "neden": why,
        "permit": {"particulars": {"basvuru_tarihi": day}},
        "building": building(party, bearing),
        "expect": {"findings": findings, "rules": {"PAİY 39/4": [r39, versions[0]], "BYKHY 25/1": [r251, versions[1]],
                                                   "BYKHY 25/2": [r252, versions[2]]}},
    }


def f(code, state=None, has=()):
    out = {"code": code}
    if state:
        out["state"] = state
    if has:
        out["has"] = list(has)
    return out


OPEN39, FIRE, HOLE = "paiy_party_wall_opening", "bykhy_party_fire_wall", "bykhy_party_wall_opening"


def cases():
    both = lambda w, *o: [side("k00", [w]), side("k01", [wall(2, w.get("fireRating"))], o)]  # noqa: E731
    out = [
        case("yok", "Vaziyet planında bitişik kenar yok → üç madde de kapsam dışı.", [], [], OUT_OF, OUT_OF, OUT_OF),
        case("tam", "2. kenar bitişik; iki katta EI 90 duvar, açıklık yok → üç madde geçer.",
             both(wall(1, "EI 90")), [], PASSED, PASSED, PASSED),
        case("pencere-sinifsiz", "1. kattaki bitişik duvarda sınıfsız pencere → 39/4 kalır, 25/2 eksik veri.",
             both(wall(1, "EI 90"), opening(11, "window")),
             [f(OPEN39, has=["2. kenar (bitişik), 1. kat", "1 pencere"]), f(HOLE, MISSING, ["pencere", "beyan edilmemiş"])],
             FAILED, PASSED, MISSING),
        case("kapi-yangin", "Bitişik duvarda EI2 60-S200C5 kapı: 39/4 kalır; 25/2'nin istisnası (E 45, C, S) tutar, "
             "kaçınılmazlığı uzman incelemesi.",
             both(wall(1, "EI 90"), opening(11, "door", "EI2 60-S200C5")),
             [f(OPEN39, has=["1 kapı"]), f(HOLE, EXPERT, ["kaçınılamadığı"])], FAILED, PASSED, EXPERT),
        case("kapi-cs-yok", "Bitişik duvarda EI 60 kapı (C ve S yok): 25/2 kendiliğinden kapanma ve duman sızdırmazlık "
             "ister → eksik veri.",
             both(wall(1, "EI 90"), opening(11, "door", "EI 60")),
             [f(OPEN39), f(HOLE, MISSING, ["kendiliğinden kapanma (C) ve duman sızdırmazlık (S)"])], FAILED, PASSED,
             MISSING),
        case("kapi-az", "Bitişik duvarda E 30 kapı: yarı süre (E 45) değil → 25/2 kalır.",
             both(wall(1, "EI 90"), opening(11, "door", "E 30-S200C5")),
             [f(OPEN39), f(HOLE, has=["en az E 45 değil"])], FAILED, PASSED, FAILED),
        case("bosluk", "Bitişik duvarda kapısız, penceresiz boşluk: pencere ya da kapı değil (39/4 geçer); 25/2 yangın "
             "duvarında boşluk istemez → kalır.",
             both(wall(1, "EI 90"), opening(11, "gap")), [f(HOLE, has=["boşluk"])], PASSED, PASSED, FAILED),
        case("pencere-e60", "Bitişik duvarda E 60 pencere: 39/4 kalır; 25/2'de süre yeter, sabit ışık penceresi olduğu ve "
             "kaçınılmazlığı uzman incelemesi.",
             both(wall(1, "EI 90"), opening(11, "window", "E 60")),
             [f(OPEN39, has=["1 pencere"]), f(HOLE, EXPERT, ["sabit olduğu"])], FAILED, PASSED, EXPERT),
        case("iki-pencere-kapi", "Bitişik duvarda iki pencere ve bir kapı: tek 39/4 bulgusu, üç 25/2 bulgusu.",
             both(wall(1, "EI 90"), opening(11, "window"), opening(12, "window"), opening(13, "door")),
             [f(OPEN39, has=["2 pencere ve 1 kapı"]), f(HOLE, MISSING), f(HOLE, MISSING), f(HOLE, MISSING)],
             FAILED, PASSED, MISSING),
        case("duvar-sinifsiz", "Bitişik duvarın sınıfı beyan edilmemiş → 25/1 eksik veri.",
             [side("k00", [wall(1)])], [f(FIRE, MISSING, ["beyan edilmemiş"])], PASSED, MISSING, PASSED),
        case("duvar-ei60", "Bitişik duvar EI 60: 25/1 en az 90 dakika ister → kalır.",
             [side("k00", [wall(1, "EI 60")])], [f(FIRE, has=["EI 60", "en az EI 90 değil"])], PASSED, FAILED, PASSED),
        case("tasiyici-ei90", "Taşıyıcı beyan edilmiş bitişik duvar EI 90 (R yok) → en az REI 90 değil, kalır.",
             [side("k00", [wall(1, "EI 90")])], [f(FIRE, has=["en az REI 90 değil"])], PASSED, FAILED, PASSED,
             bearing=(uid(1),)),
        case("tasiyici-rei90", "Taşıyıcı bitişik duvar REI 120 → 25/1 geçer.",
             [side("k00", [wall(1, "REI 120")])], [], PASSED, PASSED, PASSED, bearing=(uid(1),)),
        case("iki-kenar", "2. ve 4. kenar bitişik: her kenarın duvarı ayrı okunur; 4. kenarınki EI 30 kalır.",
             [side("k00", [wall(1, "EI 90")]), side("k00", [wall(3, "EI 30")], edge=4)],
             [f(FIRE, has=["4. kenar (bitişik), Zemin kat", "EI 30"])], PASSED, FAILED, PASSED),
    ]
    # An application before 39/4's first text (1.10.2017): 25 is read, 39/4 expert review.
    old = case("2017-basvuru", "Başvuru 1.6.2017: 39/4'ün kural setindeki ilk hâli (1.10.2017) sonra → uzman incelemesi; "
               "25/1 ve 25/2 okunur.", both(wall(1, "EI 90"), opening(11, "window", "E 60")),
               [f("regulation_text_unknown", EXPERT, ["1.10.2017"]), f(HOLE, EXPERT)], EXPERT, PASSED, EXPERT,
               day="01.06.2017")
    old["expect"]["rules"]["PAİY 39/4"] = [EXPERT, None]
    out.append(old)
    older = case("2006-basvuru", "Başvuru 1.6.2006: üç maddenin de ilk hâli sonra → üçü de uzman incelemesi.",
                 both(wall(1, "EI 90")), [f("regulation_text_unknown", EXPERT)] * 3, EXPERT, EXPERT, EXPERT,
                 day="01.06.2006")
    older["expect"]["rules"] = {"PAİY 39/4": [EXPERT, None], "BYKHY 25/1": [EXPERT, None], "BYKHY 25/2": [EXPERT, None]}
    out.append(older)
    return out


def document():
    return {
        "format": "kentos.regulationCases",
        "version": 1,
        "about": ("Komşu binaya bitişik duvarın bağımsız durumları (docs/adr/8142): beklenen her sonuç Planlı Alanlar İmar "
                  "Yönetmeliği 39/4 ile Binaların Yangından Korunması Hakkında Yönetmelik 25/1 ve 25/2'nin derlenmiş "
                  "resmî metinlerinden elle çıkarıldı (scripts/fixtures/party_wall_cases.py); kuralın kodundan "
                  "üretilmedi. Olgular KentOS'un ölçüsüdür (building.party: vaziyet planının bitişik kenarı boyunca "
                  "duvarlar ve açıklıklar)."),
        "cases": cases(),
    }


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.is_file() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırıp farkı okuyun.")
        print(f"{OUT.relative_to(ROOT)}: {len(document()['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(document()['cases'])} durum yazıldı.")


if __name__ == "__main__":
    main()
