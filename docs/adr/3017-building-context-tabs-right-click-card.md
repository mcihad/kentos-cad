# 3017 — Bina nesnesinin bağlamı: sekme, sağ tık ve kart

Tarih: 2026-11-08. Durum: kabul (M17, ADV-09; plan S11, S15).

## Sorun

- **Duvar sekmesinde Duvarı kaydır yoktu** (araştırma A1 §1.4). Duvarın en sık işi Mimari › Düzenle ▾'de kalmıştı. Stil ve
  malzeme yalnız Stili değiştir'di. Bağlar (ADR 0770'in mimari bağları) sekmede hiç yoktu.
- **Mahal sekmesinin on bir düğmesi tek paneldeydi.** "Tavan" tavan ızgarasıydı, "Mahal" mahal çizelgesiydi. "İç nokta seç"
  mahal dediğini söylemiyordu.
- **Mimari nesne sekmesinde Özellikler yoktu.** "Denetle" neyi denetlediğini söylemiyordu (Etiketleri denetle).
- **Sağ tık menüsü geneldi** (`drawing_menus.rs`): duvarda Taşı, Kopyala, pano, çizim sırası, Koordinat listesi vardı. Duvarın,
  kapının ya da mahalin kendi aracı yoktu.
- **Üzerine gelme kartı geneldi:** duvar "Alan 0,60 m² · Uzunluk 3,000 m" diyordu. Ayak izinin alanını kimse sormaz; kalınlık,
  stil ve kat yoktu. Mahal kartında alan bile yoktu (genel ölçü mahalin alanını hesaplamıyordu).

## Araştırma (8 Kasım 2026)

- **Revit yardımı** (*Editing the Profile of a Wall*, *Attach Walls to Other Elements*): duvar seçilince açılan Modify | Walls
  bağlam sekmesi duvarın kendi işlerini taşır. Mode panelinde Edit Profile ve Reset Profile, Modify Wall panelinde Attach
  Top/Base ve Detach Top/Base vardır. Revit 2023'ün resmî arayüz resminde sekme satırında "Modify | Walls" görünür
  (`.run/gui-evidence/mimari-m15/referans/revit-2023-arayuz-GUID-453F6BD3.png`).
- **ADR 0622 §6:** seçim varken sağ tık AutoCAD'in düzenleme menüsüdür. Bu ADR o menünün önüne nesnenin kendi bölümünü koyar,
  menüyü değiştirmez.
- **Araştırma A1 S11:** "Nesne türüne göre sağ tık bölümü: katalogdaki `ctx.*` panelleri sağ tıkta da listelensin." Mimari kart
  için duvar (stil, kalınlık, uzunluk, kat), mahal (no, ad, alan), kapı (tip, en, no) istenir.

## Karar

- **Duvar sekmesi** (`scripts/ui/ribbon.py`):
  - Duvar paneli: Duvarı kaydır büyük; Duvar birleşimi, Duvarı böl, Tek duvara çevir, Duvarı uzat, Duvarı buda, Duvarı çevir,
    Özellikler.
  - Stil: Stili değiştir, Duvar stilleri, Malzemeler.
  - Bağ: Kısıtları göster ("Göster") ve Gevşeterek düzenle ("Gevşet"). Mimari bağlar kısıttır (ADR 0770).
  - Açıklık: Kapı, Pencere, Boşluk ▾. Belgele: Duvar ölçüleri büyük, Etiket, Birleşimleri denetle.
- **Mahal sekmesi:** Mahal (Özellikler, Mahali böl, Mahali yeniden sınırla, Mahal kaynakları, Mahal numarala, Mahalleri
  birleştir) · Kaplama (Döşeme deseni, Tavan ızgarası) · Belgele (Metraj, Etiket, Mahal çizelgesi).
- **Mimari nesne sekmesi:** Özellikler eklendi.
- **Türkçe düzeltmeler:**
  - Mahal çizelgesi'nin kısa adı "Mahal" → yok; düğme "Mahal çizelgesi" der.
  - Tavan ızgarası'nın kısa adı "Tavan" → yok; düğme "Tavan ızgarası" der.
  - Etiketleri denetle'nin kısa adı "Denetle" → yok; düğme "Etiketleri denetle" der.
  - "İç nokta seç" → "Mahali yeniden sınırla" (kısa adı "Yeniden sınırla"). Aracın adı, istemi ("Yeni iç noktaya tıklayın"),
    Öznitelikler'in satırı ve iş kuralı iletisi birlikte değişti.
  - Öznitelikler'in Kat listesindeki "Katlar…" → "Katları düzenle…". Durum çubuğunun Kat menüsü zaten böyle diyordu. Komutun
    adı M15'te "Mimari ayarları" olmuştu; listede ne yapacağını söyleyen ad kaldı.
- **Sağ tık** (`App::building_tools`): seçim bir bina sekmesini açıyorsa (duvar, açıklık, mahal, döşeme, mimari nesne), menünün
  ilk bölümü o sekmenin Seçim ve Düzenle'den sonraki ilk panelidir.
  - Başlığı sekmenin adıdır (Duvar, Açıklık, Mahal). Tutamacın bölümü varsa ondan sonra, Yinele'den önce gelir.
  - Ailenin her aracı yazılır; Nesne özellikleri bölümü kapatır.
  - Kaynak şeridin kendisidir: sekme değişirse menü de değişir, ikinci bir liste tutulmaz.
  - Karışık seçimde (duvar ve kapı) bölüm yoktur, Seçim sekmesi gibi.
- **Kart** (`hover_card::building_rows`): değerler çizelge ve etiketlerin okuduğu yerden gelir (`Document::arch_value`), kart ile
  etiket iki şey söylemez.
  - Duvar: Stil, Kalınlık, Kat, sonra Uzunluk. Alan yazılmaz.
  - Açıklık: başlığı biçimidir (Kapı, Pencere, Niş …). Satırları Tür (varsa), Genişlik, Numara.
  - Mahal: Numara, Ad, Kat, Alan.

## Ölçüm

- **Gerçek pencere** (`fixtures/interaction/real/mimari-baglam.txt`, 1440×900, sahibin ayarlarının kopyası):
  - Duvar ▾ › Dikdörtgen duvar ile iki köşe tıklandı (Dış duvar 25), içine Mahal kondu.
  - Duvar seçilince Duvar sekmesinde Kaydır büyük. Sağ tıkta "Duvar" başlığı altında 8 satır (önce 0).
  - Mahal adından seçilince Mahal sekmesi üç panel. Sağ tıkta "Mahal" başlığı altında 6 satır.
  - Kart: duvarda "Stil Dış duvar 25 · Kalınlık 0,250 m · Kat Zemin kat · Uzunluk 28,912 m" (önce "Alan 0,60 m² · Uzunluk
    3,000 m"). Mahalde "Kat Zemin kat · Alan 51,81 m²" (önce alan yoktu).
  - Resimler: `.run/gui-evidence/mimari-m17/once/`, `sonra/`, `m17-once.png`, `m17-sonra.png`.
- **Testler** (`building_context_tests.rs`):
  - `the_selection_s_building_panel_opens_the_right_click_under_its_tab_s_name`: duvar, kapı ve mahal; mahalin çizelgesi
    bölüme girmez; karışık seçimde bölüm yok.
  - `the_duvar_tab_holds_duvari_kaydir_its_style_and_its_bonds`.
  - `the_card_says_what_an_architect_asks_of_a_wall_a_door_and_a_room`: duvarda Alan yok, kapının başlığı Kapı, mahalde Alan var.

## Sonraki adımlar

- Sağ tık menüsü tutamacın bölümüyle birlikte 1440×900'de pencereden uzun. KentOS UI'nin menüsü kaydırıyor; tutamacın Taşı,
  Döndür, Ölçekle ve Aynala'sı genel bölümle yineleniyor (bu ADR'den önce de vardı).
- Kolon, merdiven ve rampanın kendi araçları (Mimari nesne sekmesi bugün yalnız Aksları numarala, Etiketleri denetle,
  Başvurular).
- Seçimin yanında küçük eylem şeridi (A1 S11'in üçüncü önerisi).
