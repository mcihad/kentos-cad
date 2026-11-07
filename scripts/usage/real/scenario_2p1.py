"""Writes fixtures/interaction/real/daire-2p1.txt: the 2+1 flat of fixtures/interaction/v1/acceptance-flat.json done
as a hand does it in the real window (docs/adr/3000): tools from the Mimari ribbon, values in the tool panel's
fields, points with the mouse, snapping and grid as the owner's settings have them. The trace's world metres
become screen pixels of the view Home gives after the outer walls (measured on 7 November 2026 in a 1440×900
screen: 1 m = 66.47 px, the plan's (0, 0) at (160.4, 741.4)); a changed layout moves them, so measure again.
`python3 scripts/usage/real/scenario_2p1.py`"""
import pathlib
PX, OX, OY = 66.47, 160.4, 741.4
def s(x, y):
    return f"{round(OX + PX * x)} {round(OY - PX * y)}"
L = []
def a(line): L.append(line)
def field(y, value, what):
    a(f"click 1300 {y}   # panel: {what}"); a("key ctrl+a"); a(f"type {value}"); a("key Return"); a("wait 200")
a("fit KentOS Mimari"); a("wait 1500")
a("click 353 343   # Başlangıç: Yeni proje"); a("wait 1500")
a("click 589 20    # Mimari sekmesi"); a("wait 500")
# Dış kontur: Dikdörtgen duvar, iç net, 25 cm
a("click 115 84    # araç: Dikdörtgen duvar"); a("wait 400")
field(655, "0.25", "Kalınlık")
a("move 500 450"); a("type 0,0"); a("key Return"); a("type 11,8"); a("key Return"); a("wait 500")
a("key Escape"); a("key Home"); a("wait 800"); a("shot r01-kontur")
# Bölmeler: Ölçülü bölme, 15 cm
a("click 234 56    # araç: Ölçülü bölme"); a("wait 400")
field(655, "0.15", "Kalınlık"); field(684, "4.5", "Uzaklık")
a(f"click {s(-0.1, 2.0)}   # referans: sol duvar"); a(f"move {s(2.0, 2.0)}"); a(f"click {s(2.0, 2.0)}   # yan: sağı")
a("wait 400"); a("shot r02-bolme-1")
L.append("# 2. bölme: alt duvardan 4.5, bölme 1 ile sağ duvar arasında (Uçlar)")
a("click 1300 771   # panel: Uçlar"); a("wait 400"); a("click 1305 835   # İki duvar arasında"); a("wait 300")
a(f"click {s(2.5, -0.1)}   # referans: alt duvar"); a(f"click {s(4.575, 3.0)}   # uç: bölme 1"); a(f"click {s(11.1, 3.0)}   # uç: sağ duvar")
a(f"click {s(6.0, 2.0)}   # yan"); a("wait 300")
L.append("# 3. bölme: bölme 1'den 3.5")
field(684, "3.5", "Uzaklık")
a(f"click {s(4.625, 2.0)}   # referans: bölme 1"); a(f"click {s(6.5, -0.1)}   # uç: alt duvar"); a(f"click {s(6.5, 4.575)}   # uç: bölme 2")
a(f"click {s(6.0, 2.0)}   # yan"); a("wait 300")
L.append("# 4. bölme: sağ duvardan 3.0")
field(684, "3", "Uzaklık")
a(f"click {s(11.1, 6.0)}   # referans: sağ duvar"); a(f"click {s(9.5, 4.575)}   # uç: bölme 2"); a(f"click {s(9.5, 8.1)}   # uç: üst duvar")
a(f"click {s(9.5, 6.0)}   # yan"); a("wait 300")
a("key Escape"); a("key Escape"); a("wait 300"); a("shot r04-bolmeler")
# Kapılar
a("click 452 75    # araç: Kapı"); a("wait 400")
for x, y, w in ((4.575, 6.3, "salon-koridor"), (6.4, 4.575, "yatak 1"), (9.7, 4.575, "yatak 2"), (7.925, 6.3, "banyo")):
    a(f"click {s(x, y)}   # kapı: {w}"); a("wait 200")
field(713, "1.0", "Genişlik")
a(f"click {s(6.2, 8.1)}   # giriş kapısı"); a("wait 300")
a("key Escape"); a("key Escape")
# Pencereler
a("click 509 75    # araç: Pencere"); a("wait 400")
for x, y in ((2.25, -0.1), (6.4, -0.1), (9.7, -0.1), (11.1, 6.3), (-0.1, 4.0)):
    a(f"click {s(x, y)}   # pencere"); a("wait 200")
a("key Escape"); a("key Escape"); a("wait 300"); a("shot r05-aciklik")
# Mahaller
a("click 214 111   # araç: Mahal"); a("wait 400")
for name, x, y in (("Salon", 2.0, 2.0), ("Yatak odası 1", 6.0, 2.0), ("Yatak odası 2", 9.5, 2.0), ("Koridor", 6.0, 6.0), ("Banyo", 9.5, 6.0)):
    field(771, name, "Ad"); a(f"click {s(x, y)}   # mahal: {name}"); a("wait 300")
a("key Escape"); a("key Escape"); a("wait 300"); a("shot r06-mahaller")
# Mutfak dizisi
a("click 671 111   # araç: Mutfak"); a("wait 400")
a(f"click {s(0.05, 7.95)}"); a(f"click {s(3.0, 7.95)}"); a(f"click {s(1.5, 7.0)}"); a("wait 300")
a("key Escape"); a("key Escape"); a("wait 300"); a("shot r07-mutfak")
# Duvar ölçüleri
a("click 1204 56   # Ölçü ▾"); a("wait 300"); a("click 1275 90   # araç: Duvar ölçüleri"); a("wait 400")
a(f"click {s(5.5, -0.1)}"); a(f"click {s(5.5, -0.9)}"); a("wait 300"); a("key Escape"); a("key Escape")
# Döşeme
a("click 340 56    # araç: Döşeme"); a("wait 400"); a(f"click {s(2.0, 2.0)}"); a("wait 300"); a("key Escape"); a("key Escape")
# Hepsini etiketle
a("click 1225 132  # Belgele ▾"); a("wait 300"); a("click 1278 162   # araç: Hepsini etiketle"); a("wait 500")
a("key Return"); a("wait 400"); a("key Escape")
# Tasarım değişikliği: yatak odası 1'in doğu bölmesi seçilir, geçici ölçünün batı değeri 3.60 yapılır
a(f"click {s(8.225, 2.0)}   # bölme 3"); a(f"move {s(7.925, 2.0)}"); a("wait 500"); a("shot r08-gecici-olcu")
a("click 586 587   # geçici ölçü: batı 3.500"); a("wait 300"); a("type 3.6"); a("key Return"); a("wait 400")
a("key Escape"); a("wait 300"); a("shot r09-degisiklik")
# Çizelgeler: önce uzaklaş, boş yere koy
a("wheel 526 475 -3"); a("wait 500"); a("shot r10-uzak")
a("click 1294 56   # Çizelge ▾"); a("wait 300"); a("click 1180 90   # araç: Kapı ve pencere çizelgesi"); a("wait 400"); a("click 720 330"); a("wait 400")
a("click 589 20    # Mimari sekmesine dön: tablo yerleşince şerit Tablo sekmesine geçti"); a("wait 400")
a("click 1294 56   # Çizelge ▾"); a("wait 300"); a("click 1180 118  # araç: Mahal çizelgesi"); a("wait 400"); a("click 720 540"); a("wait 600")
a("key Escape"); a("key Home"); a("wait 800"); a("shot r11-cizelgeler")

if __name__ == "__main__":
    path = pathlib.Path(__file__).resolve().parents[3] / "fixtures/interaction/real/daire-2p1.txt"
    head = ["# 2+1 daire gerçek pencerede (docs/adr/3000; üretici scripts/usage/real/scenario_2p1.py).",
            "# Oynatma: scripts/usage/real/session.sh fixtures/interaction/real/daire-2p1.txt .run/gui-evidence/gercek-2p1 --modul mimari"]
    path.write_text("\n".join(head + L) + "\n", encoding="utf-8")
    print(path, len(L), "satır")
