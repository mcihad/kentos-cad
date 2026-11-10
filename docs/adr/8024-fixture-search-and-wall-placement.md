# 3024 — Donatı araması ve duvara yaslı yerleşim

Tarih: 2026-11-08. Durum: kabul (M24, ADV-12; plan S13; ADV-14'ün "donatı ve blok katmanları").

## Sorun

- **"lavabo"dan duvara yaslı lavaboya altı eylem:** Donatı ▾ › Kitaplık (kitaplığın 38 bloğunun hepsi çizime alınır),
  Bloklar panelinde arama, karta çift tık, Duvara hizala için H, duvarın yanına tık.
- **Menü araması bloğu bulmuyordu.** "lavabo" yalnız Kitaplık komutunu buluyordu (KentOS Mimari'nin LAVABO adı, ADR 8002).
- **Gündelik sözcükler bir şey bulmuyordu:** "karyola", "tuvalet", "duşakabin" (Bloklar panelinin araması ad, açıklama,
  kategori ve etiketlere bakıyor; eş anlamlıyı bilmiyor).
- **Donatı etkin katmana yazılıyordu.** ADR 8008 blok yerleştirmeyi kapsam dışı bırakmış, donatıların TEFR'e bu maddede
  bağlanacağını yazmıştı.

## Araştırma (8 Kasım 2026)

- **Sweet Home 3D kılavuzu** (şekil 8, "Adding doors, windows and furniture"):
  - "When magnetism is active, a piece of furniture is automatically rotated so its back face lies along the wall upon
    which you released the mouse cursor."
  - Katalog kategorili ve aranabilir.
  - Resim: `.run/gui-evidence/mimari-m24/referans/sh3d-figure8-mobilya-duvara.png`.
- **AutoCAD 2024 "Smart Blocks: Placement":** blok, daha önce yerleştirildiği yerlere göre öneriyle yerleşir; konum, dönüş
  ve ölçek yakındaki geometriye uyar.
- **KentOS'ta parçalar hazırdı:**
  - Kitaplığın blokları kategorili ve etiketli (ADR 0317 §4, 0702): Islak hacim, Mobilya, Mutfak.
  - Taban noktaları arka kenarda, önleri +y (ADR 0702).
  - Blok ekle'nin Duvara hizala'sı var, ama kapalı başlıyordu.
  - Tefriş katmanı EK-2 Tablo 2.3'ün TEFR'i (ADR 8008).

## Karar

- **Menü araması kitaplığın donatılarını da bulur** (`fixture_search.rs`, `ribbon_search::Hit::Fixture`):
  - Kapsam: Islak hacim, Mobilya ve Mutfak kategorileri. Kapı, pencere ve işaretlerin kendi araçları var.
  - Satır: bloğun adı ve "Donatı › Islak hacim › Lavabo".
  - Sıra:
    - adın kendisi (−1) Mimari'nin LAVABO adının komutundan önce gelir;
    - ad o sözcükle başlıyorsa 1,5; ad içinde geçiyorsa 2,5;
    - etiket 3, kategori 3,5;
    - en az iki harf.
  - Blok yerleştirmeyen programda (bir modülün `tool.blockInsert`'ü yoksa) çıkmaz.
- **Enter:**
  - Çizimde o adlı blok varsa o kullanılır.
  - Yoksa yalnız o tanım kitaplıktan alınır (Blokları al, bir geri alma adımı). Kitaplığın dosyası aynen süzülür; sürümü
    ve dinamik parametreleri korunur.
  - Blok ekle onunla başlar. İleti: "“Lavabo” kitaplıktan alındı. Duvarın yanına tıklayın: arkası duvara oturur, önü
    odaya bakar."
- **Eş anlamlılar** (`fixture_search::WORDS`):
  - karyola, baza, somya → yatak;
  - elbise dolabı → gardırop etiketi;
  - tuvalet, alafranga, rezervuar, WC → klozet;
  - eviye → evye; duşakabin, duş kabini → duş; jakuzi → küvet;
  - çekyat, divan, sedir → kanepe;
  - pisuar → pisuvar; komidin → komodin; makina → makine; televizyon → TV;
  - banyo dolabı → dolaplı lavabo; set üstü → ocak; kitap → kitaplık.
  - Üç harfle başlanan söz de sayılır (karyo).
  - Bloklar panelinin araması da bu sözleri tanır.
  - "gardırop" bütün dolapları değil, etiketiyle yalnız gardırobu bulur.
- **KentOS Mimari'de Duvara hizala açık başlar** (`arch_start.rs`; öbür programlarda kapalı). Duvar yüzüne 40 px
  yakındayken bloğun arkası yüze oturur, önü odaya bakar (ADR 0702). H kapatır.
- **Donatı tefriş katmanına yazılır** (`block_insert::fixture_kind`, `arch_layer::Kind::Furniture`):
  - Mobilya → MM-TEFR-MOBILYA; Islak hacim → MM-TEFR-VITRIFIYE; Mutfak → MM-TEFR-MUTFAK (Mutfak dizisinin katmanı).
  - EK-4 Tablo 4.1'in üçüncü alanı "tip"tir. Değer kitaplığın kategorisinden gelir. Islak hacim için araştırma notlarının
    ve planın "vitrifiye" sözü kullanıldı.
  - Katman ilk donatıyla, aynı geri alma adımında açılır. Araç panelinin Katman satırı söyler.
  - Başka bloklar (işaret, antet, çizimin kendi blokları) etkin katmanda kalır.
- **Şeritte kategori düğmeleri yok.** Bloklar panelinin kategori seçicisi ve kart görünümü (ADR 0317 §4) zaten kategoriye
  göre gösteriyor. Donatı ▾'ye beş kategori eklemek beş yeni ikon ister (ADR 8016'nın kuralı). Sonraki adımlarda.

## Ölçüm

- **Testler:**
  - `fixture_search::tests`:
    - "lavabo" → Lavabo, Dolaplı lavabo;
    - "karyola" → iki yatak, sonra komodin;
    - "tuvalet" → Klozet önce, Asma klozet de;
    - "eviye" → Evye; "gardırop" → Dolap; "duşakabin" → Köşe duş;
    - tek harf ve "kapı", "kuzey" boş.
    - Panelin araması: "karyola" yatağı bulur, kanepeyi bulmaz.
  - `fixture_place_tests::lavabo_enter_and_a_click_by_a_wall_place_the_washbasin_against_it`:
    - Duvar kuzeye uzanıyor; "lavabo" yazılınca ilk satır Lavabo "Donatı › Islak hacim › Lavabo".
    - Enter: araç Blok ekle, çizimin blokları +1 (38 değil), Duvara hizala açık.
    - Yüzün 12,5 cm doğusuna tık: taban noktası x = 0,125 (yüzde), önü doğuya.
    - Katmanı MM-TEFR-VITRIFIYE; tek geri alma bloğu ve katmanı birlikte götürür.
  - `fixture_place_tests::the_other_programs_keep_duvara_hizala_off`.
  - `arch_layer`: MM-TEFR-MUTFAK, MM-TEFR-VITRIFIYE EK-4'e uyar.
- **Gerçek pencere** (`fixtures/interaction/real/donati-arama.txt`, 1440×900, sahibin ayarlarının kopyası):
  - Oda ile oda çizildi; menü aramasına "lavabo": Lavabo, KentOS blok kitaplığı, Dolaplı lavabo.
  - Enter: panelde Katman MM-TEFR-VITRIFIYE, komut çubuğunda "Duvara hizala: açık".
  - Batı duvarının yanında etiket "dönüş -90.0° duvara göre"; tık: lavabo duvara yaslı yerleşti.
  - Kartı "Blok · MM-TEFR-VITRIFIYE". "karyola": Çift kişilik yatak, Tek kişilik yatak, Komodin.
  - **Eylem sayısı:** "lavabo" yazmak, Enter, tık = 3 (ADV-12'nin ilk kabulü: ≤ 3). Önce 6'ydı.
  - Resim: `.run/gui-evidence/mimari-m24/m24-once-sonra.png`.

## Sonraki adımlar

- Kullanım alanları ve çakışma (M25): kapı kanadı, klozet önü, tezgâh önü.
- Donatı ▾'de kategoriler (Islak hacim, Yatak ve oturma, Mutfak, Çalışma, Dış mekân) ve paneli o kategoriyle açmak; yeni
  ikonlar.
- Galeriden çizime sürükle-bırak (Sweet Home 3D gibi).
- Komut satırında donatı adıyla yerleştirme: bugün LAVABO, KLOZET adları Kitaplık komutunu çalıştırır (ADR 8002).
