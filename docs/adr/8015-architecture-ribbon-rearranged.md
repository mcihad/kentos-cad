# 8015 — Mimari şeridin yeniden düzeni

Tarih: 2026-11-08. Durum: kabul (M15, ADV-09; plan S6, G13).

## Sorun

- **Sekmede 37 görünen düğme vardı, 78 komut taşıyordu.** Katalogda büyük olanlar Duvar, Dikdörtgen duvar, Kapı, Pencere ve
  Ölçü idi. 1440 px'te Ölçü küçülüyor, Proje paneli adıyla katlanıyordu (`.run/gui-evidence/mimari-m15/once/`).
  - Mahal küçüktü.
  - Çiz paneli dört duvar aracını, Mahal'i, Döşeme'yi, Toplu mahal bul'u ve Mahal ayırıcı'yı bir arada tutuyordu.
- **Duvarın dört yolu dört ayrı düğmeydi** (Duvar, Dikdörtgen duvar, Ölçülü bölme, Çizgiden duvar). İkisi büyüktü, ana
  araçlara yer kalmıyordu.
- **Donatı diye bir düğme yoktu.** Yerleştir'deki küçük Mutfak yalnız mutfak dizisini yerleştiriyordu. Yatak, masa, klozet ve
  lavabo KentOS blok kitaplığındaydı, Mimari sekmesinde yoktu (Ekle › Kitaplık).
- **Kat değiştirmek şeritte yoktu.** Üst kat, Alt kat ve kat görünümü durum çubuğunun Kat hücresinde ve Görünüm sekmesindeydi;
  ADV-09 "şeritte etkin kat ve evre seçicisi" diyor. Evre ▾ vardı, Kat yoktu.
- **"Katlar" adı işini söylemiyordu.** Düğme yalnız kat listesini değil, Proje ayarları'nın bütün Mimari bölümünü açıyordu:
  duvarların görünüşü, kesme yüksekliği ve çizgi kalınlıkları da oradadır.
- **G13:** şerit etiket uzunluğuna duyarlıydı. 1440 px'te tek bir etiket üç harf kısalınca Proje ▾ açılıyor, büyük düğmeler
  küçülüyordu.

## Araştırma (8 Kasım 2026)

- **Revit 2023 yardımı, *Parts of the User Interface*** (resmî resim:
  `.run/gui-evidence/mimari-m15/referans/revit-2023-arayuz-GUID-453F6BD3.png`):
  - Architecture sekmesinin Build panelinde **Wall ▾** tek düğmedir, türleri açılır listesindedir; Wall ve Door büyüktür.
  - Window, Component ▾, Column ▾, Roof ▾, Ceiling ve Floor ▾ küçüktür.
  - Room & Area panelinde Room, Room Separator, Tag Room ▾ ve Area ▾; Datum panelinde Level ve Grid vardır.
  - Sekmede yaklaşık 35 düğme vardır, üçü büyüktür (Modify, Wall, Door).
  - *Create Walls* (Revit Get Started): "Click Architecture tab › Build panel › (Wall)."
- **DESIGN.md §7.3.1:**
  - boyut anlamdan gelir; panelde en çok dört büyük düğme olur;
  - bir aile ve yöntemleri tek bölünmüş düğmedir, üst parçası son seçileni çalıştırır;
  - seyrek araçlar panelin ▾'sindedir;
  - panelin başlığı her zaman görünür (sahibin kuralı, 7 Kasım 2026).
- **Plan ADV-09:**
  - Duvar, Oda, Kapı, Pencere, Mahal ve Donatı büyük düğmedir; Duvar ▾: duvar, dikdörtgen, ölçülü bölme, çizgiden.
  - Görünen düğme en çok 30, menü başına en çok 6 öğe; şeritte etkin kat ve evre seçicisi bulunur.

## Karar

- **Paneller:** Duvar · Yerleştir · Mahal · Donatı · Düzenle · Belgele · Proje (önce Çiz · Yerleştir · Düzenle · Belgele · Proje).
- **Büyük düğmeler bir planın ana işleridir:** Duvar ▾, Kapı, Pencere, Mahal ▾, Donatı ▾ ve Ölçü ▾.
  - Oda aracı henüz yok (M21); onun yerine Ölçü büyük kalır.
  - **Duvar ▾:** Duvar, Dikdörtgen duvar, Ölçülü bölme, Çizgiden duvar.
  - **Mahal ▾:** Mahal, Toplu mahal bul, Mahal ayırıcı. Döşeme yanında küçük durur.
  - **Donatı ▾:** Mutfak (mutfak dizisi) ve Kitaplık (KentOS blok kitaplığı: mobilya, ıslak hacim).
  - **Etiket ▾:** Etiket ve Hepsini etiketle, Revit'in Tag panelinde Tag All gibi. Hepsini etiketle önce Belgele ▾'deydi (S6'nın
    "sık iş menüde" dediklerinden).
- **Kat ▾ (Proje paneli):**
  - Üst kat ve Alt kat (Ctrl+↑, Ctrl+↓);
  - Kat görünümü: Bütün katlar, Yalnız etkin kat, Etkin kat ve alt kat; işaretli olan şimdiki görünümdür.
  - Durum çubuğunun Kat hücresiyle ve Görünüm sekmesiyle aynı komutlardır.
  - Katların kendisi yanındaki Mimari ayarları düğmesindedir; menüde ikinci kez yazılmaz.
- **Türkçe düzeltme:** "Katlar" → **"Mimari ayarları"**. Açıklaması bölümde ne olduğunu sayar: katlar, duvarların görünüşü, kesme
  yüksekliği, çizgi kalınlıkları.
- **Seyrek iş panelin ▾'sine indi:**
  - Düzenle ▾: İç nokta seç (mahalin sınırını yeniden bul), Mahal kaynakları, Açıklık dağıt;
  - Belgele ▾: Revizyon işaretle ve Revizyon listesi (Revizyon ▾ düğmesi kalktı; revizyon pafta yayımlandıktan sonra gelir);
  - Proje ▾: Bina programı ve Mimari şablonu.
- **Duvarı kaydır, Uzat ve Buda Düzenle ▾'de kaldı.** Seçili bir duvara uygulanırlar; M17 onları Duvar bağlam sekmesine koyar.
- **Dosya kaydedildiği sekmede açılır.** Yeni Mimari projesi Mimari sekmesiyle açılır (M8); bu madde açılışı değiştirmez.
- **Şeridi üreten betik yeniden tek kaynak.** `scripts/ui/ribbon.py` (ADR 0658) güncel değildi:
  - Harita ve Yol işleri `catalog/v1/ribbon.json`'u elle değiştirmişti. Betik çalıştırılınca 30 kadar ekleme siliniyordu (Harita işi
    paneli, GNSS arşivi, İmar planı, nokta adları …).
  - Hepsi betiğe taşındı. Betiğin çıktısı dosyayla, ayrıştırılınca birebir aynıdır; `--check` geçer. Baytta fark yalnız beş
    panelde anahtar sırasıdır.
- **`docs/cad/manifest.json` yeniden üretildi:** "Katlar" → "Mimari ayarları"; "Duvarları birleştir" → "Tek duvara çevir" (önceki
  bir düzeltmenin unutulmuş üretimi). `manifest.py --check` geçer.
- **`scripts/fixtures/ribbon_cases.py` de güncel değil.** Sekme listesi modül sekmelerini (Arazi, Güzergâh, Koridor, Plan, Ölçme,
  Mimari) ve bağlam sekmelerini tanımıyor; çalıştırılırsa onları `fixtures/shell/v1/ribbon.json`'dan siler. Mimari'nin
  panellerini taşımadığı için bu işte dokunulmadı (TODOS.md).

## Ölçüm

- **Sayılar** (`catalog/v1/ribbon.json`, Mimari sekmesi):

  | | Önce | Sonra |
  |---|---|---|
  | Görünen düğme | 37 | 27 |
  | Büyük düğme | 5 | 6 |
  | Komut (düğmelerde ve ▾ listelerinde) | 78 | 84 |
  | Panelin ▾ listelerinde | 24 | 30 |

  - Eklenen altı komut: Kitaplık ve beş kat komutu.
- **Gerçek pencere** (`fixtures/interaction/real/mimari-serit.txt`): bir çizim dosyayla açıldı, Mimari sekmesi.
  - 1440×900 ve 1100×650, açık ve koyu tema, 13 ve 18 px yazı (sahibin ayarlarının kopyasıyla).
  - 1440 px'te bütün paneller açık; 1100 px'te Düzenle, Belgele ve Proje adlarıyla katlanır, büyük düğmeler büyük kalır.
  - Duvar ▾ dört yolu, Kat ▾ beş satırı gösterir; Bütün katlar işaretli.
  - Resimler: `.run/gui-evidence/mimari-m15/once/`, `sonra-*/`, `sonra-menuler/`.
- **Testler:** `architecture_ribbon::mimari_s_tab_shows_at_most_thirty_buttons_and_its_main_tools_large` (en çok 30 düğme,
  menü başına en çok 6 öğe, panelde en çok 4 büyük, büyüklerin adları, Kat ▾'nin komutları); şeridin 1100 ve 1440 px'e, 13 ve
  18 px yazıya sığması.

## Sonraki adımlar

- M16: Mimari ikon seti (Duvar ▾, Mahal ▾, Donatı ▾ ve öbür mimari düğmelerin yüzleri).
- M17: Duvar bağlam sekmesinde Duvarı kaydır, Uzat, Buda.
- M21: Oda aracı gelince büyük düğmeler Duvar, Oda, Kapı, Pencere, Mahal ve Donatı olur.
- `ribbon_cases.py`'nin sekme listesi.
