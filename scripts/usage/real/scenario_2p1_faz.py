"""Writes fixtures/interaction/real/daire-2p1-faz.txt: the 2+1 flat of the phase gate (plan §7, docs/adr/8052) as a
hand does it today in the real window, the same flat as daire-2p1.txt (docs/adr/8000, 7 November) with today's tools:
the shell by Dikdörtgen duvar, the five rooms by Oda (walls, room and tag in one step; İç duvar 15; names from the new
project's list; corners typed), Kapı and Pencere, Mutfak, Planı ölçülendir, Döşeme, the design change on a temporary
dimension and the two schedules. 1920×1080 screen, KentOS Mimari; after Home the shell's inside corner (0, 0) is at
(284, 915) and a metre 87.6 px (measured on 12 November 2026); a changed layout moves them, so measure again.
Played with: KENTOS_REAL_SIZE=1920x1080 KENTOS_BIN=target/release/kentos-cad scripts/usage/real/session.sh
fixtures/interaction/real/daire-2p1-faz.txt OUT --modul mimari
`python3 scripts/usage/real/scenario_2p1_faz.py`"""
import pathlib

PX, OX, OY = 87.6, 284.0, 915.0  # after Home: the shell's inside corner (0, 0) and metres to pixels


def s(x, y):
    return f"{round(OX + PX * x)} {round(OY - PX * y)}"


L = []


def a(line):
    L.append(line)


def section(name):
    a(f"# {name}")


def start():
    a("fit KentOS Mimari")
    a("wait 2000")
    a("click 594 425   # Başlangıç: Yeni proje")
    a("wait 2500")


def shell():
    section("Kabuk: Dikdörtgen duvar, iç net 11 × 8, Dış duvar 25")
    a("click 31 104    # Duvar ▾")
    a("wait 500")
    a("click 94 172    # araç: Dikdörtgen duvar")
    a("wait 600")
    a("type 0,0")
    a("key Return")
    a("type 11,8")
    a("key Return")
    a("wait 800")
    a("key Escape       # Home araç açıkken komut satırına gider")
    a("key Home")
    a("wait 1200")
    a("shot f01-kabuk")


NAME_Y = {"Salon": 469, "Yatak odası": 525, "Banyo": 665, "Koridor": 833}
ROOMS = [("Salon", "0,0", "4.5,8"), ("Yatak odası", "4.65,0", "8.15,4.5"), ("Yatak odası", "8.3,0", "11,4.5"),
         ("Koridor", "4.65,4.65", "7.85,8"), ("Banyo", "8,4.65", "11,8")]


def rooms():
    section("Odalar: Oda, İç duvar 15; adı listeden, köşeleri yazılarak")
    a("click 88 75     # araç: Oda")
    a("wait 600")
    a("click 935 1032  # Stil ▾")
    a("wait 400")
    a("click 943 880   # İç duvar 15")
    a("wait 400")
    for name, c1, c2 in ROOMS:
        a("click 240 1032  # Mahal adı ▾")
        a("wait 400")
        a(f"click 236 {NAME_Y[name]}   # {name}")
        a("wait 300")
        a(f"type {c1}")
        a("key Return")
        a(f"type {c2}")
        a("key Return")
        a("wait 1000")
    a("shot f02-odalar")


def openings():
    section("Kapılar: Kapı 90, girişte Genişlik 1,0")
    a("click 153 75    # araç: Kapı")
    a("wait 600")
    for x, y, what in ((4.575, 6.3, "salon-koridor"), (6.4, 4.575, "yatak 1"), (9.7, 4.575, "yatak 2"),
                       (7.925, 6.3, "banyo")):
        a(f"click {s(x, y)}   # kapı: {what}")
        a("wait 300")
    a("key e          # Genişlik")
    a("type 1")
    a("key Return")
    a("wait 300")
    a(f"click {s(6.2, 8.125)}   # giriş kapısı")
    a("wait 300")
    section("Pencereler: Pencere 120")
    a("click 210 75    # araç: Pencere")
    a("wait 600")
    for x, y in ((2.25, -0.125), (6.4, -0.125), (9.7, -0.125), (11.125, 6.3), (-0.125, 4.0)):
        a(f"click {s(x, y)}   # pencere")
        a("wait 300")
    a("wait 300")
    a("shot f04-aciklik")


def kitchen():
    section("Mutfak dizisi: Salon'un üst duvarı boyunca 3 m")
    a("click 614 107   # Donatı ▾")
    a("wait 400")
    a("click 645 144   # araç: Mutfak")
    a("wait 600")
    a(f"click {s(0, 8)}")
    a(f"click {s(3, 8)}")
    a(f"click {s(1.5, 7)}")
    a("wait 800")
    a("shot f05-mutfak")


def dims_slab():
    section("Ölçüler: Planı ölçülendir; döşeme")
    a("click 994 56    # Ölçü ▾")
    a("wait 400")
    a("click 1070 90   # araç: Planı ölçülendir")
    a("wait 1500")
    a("key Return")
    a("wait 1200")
    a("shot f06-olcu")
    a("click 532 56    # araç: Döşeme")
    a("wait 600")
    a(f"click {s(2, 2)}")
    a("wait 600")
    a("key Escape")
    a("wait 500")
    a("shot f07-doseme")


def change():
    section("Değişiklik: Bölme 3'ü seç, geçici ölçüyü 3,6 yap")
    a(f"click {s(8.225, 2.0)}   # Bölme 3")
    a("wait 800")
    a("click 844 713   # geçici ölçü: batı 3.500")
    a("wait 400")
    a("type 3.6")
    a("key Return")
    a("wait 1500")
    a("key Escape       # seçimi bırak")
    a("wait 300")
    a("shot f08-degisiklik")


def schedules():
    section("Çizelgeler: kapı ve pencere, mahal")
    a("click 1094 56   # Çizelge ▾")
    a("wait 400")
    a("click 1195 90   # araç: Kapı ve pencere çizelgesi")
    a("wait 600")
    a(f"click {s(12.5, 8)}")
    a("wait 800")
    a("shot f085-ilk-cizelge")
    a("click 589 20    # Mimari sekmesine dön: tablo yerleşince şerit Tablo sekmesine geçer")
    a("wait 400")
    a("click 1094 56   # Çizelge ▾")
    a("wait 400")
    a("click 1170 118  # araç: Mahal çizelgesi")
    a("wait 600")
    a(f"click {s(12.5, 2)}")
    a("wait 800")
    a("key Escape")
    a("key Home")
    a("wait 1200")
    a("shot f09-cizelgeler")


SECTIONS = {"start": start, "shell": shell, "rooms": rooms, "openings": openings, "kitchen": kitchen,
            "dims": dims_slab, "change": change, "schedules": schedules}

if __name__ == "__main__":
    for f in SECTIONS.values():
        f()
    out = pathlib.Path(__file__).resolve().parents[3] / "fixtures/interaction/real/daire-2p1-faz.txt"
    head = ["# 2+1 daire, faz kapısı (docs/adr/8052; üretici scripts/usage/real/scenario_2p1_faz.py).",
            "# Oynatma: KENTOS_REAL_SIZE=1920x1080 KENTOS_BIN=target/release/kentos-cad scripts/usage/real/session.sh",
            "#   fixtures/interaction/real/daire-2p1-faz.txt .run/gui-evidence/faz-2p1 --modul mimari"]
    out.write_text("\n".join(head + L) + "\n", encoding="utf-8")
    print(f"{out.name}: {len(L)} satır")
