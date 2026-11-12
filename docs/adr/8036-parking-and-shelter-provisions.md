# 8036 — Otopark ve sığınak hesapları

Tarih: 2026-11-09. Durum: kabul (M36; ADV-22; plan S22).

## Sorun

- **Otopark ve sığınak elle hesaplanıyordu.** Mimar bağımsız bölümlerin brüt alanını metrekare cetvelinden (ADR 8029)
  okuyup Otopark Yönetmeliği'nin Ek-1 tablosuyla, Sığınak Yönetmeliği'nin kişi sayısıyla ayrı bir çizelgede
  çarpıyordu. Bölüm büyüyünce ya da dükkân eklenince çizelge eskiyordu.
- Çizimdeki otopark ve sığınak mahallerinin yeterli olup olmadığını Denetim söylemiyordu.
- **Plan M36'nın kabulü:** "Otopark ve sığınak hesapları | Çizelgeler". ADV-22: "Otopark ve sığınak hesapları: ilgili
  yönetmeliklerden, çizelge olarak."

## Araştırma (9 Kasım 2026)

Alıntılar `.run/gui-evidence/mimari-m36/m36-kaynaklar.png`.

### Otopark Yönetmeliği

mevzuat.gov.tr `MevzuatMetin/yonetmelik/7.5.24408` (.pdf ve 23.2.2026 tarihli .doc); RG 22.2.2018/30340. 13 değişiklik,
sonuncusu RG 27.12.2025/33120. Ek-1 tablosu konsolide metinde yoktur.
- RG 25.3.2021/31434 Md. 9 "Ek-1'inde yer alan tablonun 1, 2 ve 11 numaralı bölümleri" değiştirir; tablo Resmî
  Gazete'nin ekindedir ve ek indirilemedi (boş dosya döndü). Konut eşikleri ve büro oranı Çevre, Şehircilik ve İklim
  Değişikliği Bakanlığının 15.3.2021 duyurusundan alındı ("Daire büyüklüğüne göre otopark zorunluluğu hayata
  geçiyor", csb.gov.tr; W4 notlarındaki kaynak). Dükkân satırı RG 16.6.2022/31868 değişikliğinin metninden.

- **4/1-ç:** binek otosu için birim park alanı, manevra alanı dâhil en az 20 m². Park yerleri vaziyet ya da kat
  planında gösterilir.
- **5/1-j (2021):** zorunlu otopark adedi 20 ve üstündeyse en az %5'i (en az 1) elektrikli araçlara uygun düzenlenir.
- **8/3 (2021):** otopark sayısı meskenlerde bağımsız bölüm brüt alanı üzerinden, diğer yapılarda emsal hesabına konu
  alan üzerinden belirlenir. **8/4:** sonuç kesirliyse bir üst değer alınır.
- **Ek-1, konut (RG 25.3.2021/31434; Bakanlık duyurusu):** 80 m²'den küçük her 3 daireye 1; 80–120 m² her 2 daireye
  1; 120–180 m² her daireye 1; 180 m²'den büyük her daireye 2. Büro binalarında 40 m²'ye 1 olan oran 50 m²'ye 1 oldu.
- **Ek-1, dükkân, mağaza, banka (RG 16.6.2022/31868):** "Her bağımsız bölüm için en az 1 adet olmak kaydıyla 50 m2
  için (Sit alanlarında 60 m2 ye çıkarmaya idareler yetkilidir.)" 2021 haberlerindeki 40 m² bu değişiklikle eskidi.
- **10/1:** otopark yeri ayrılmadan yapı ruhsatı verilmez. **8/2:** belediyenin otopark planı daha çoğunu isteyebilir.
- 4/2 Danıştay kararıyla iptal edilmiş ve kesinleşmiştir (W4 notları); bu iş onu okumaz.

### Sığınak Yönetmeliği

mevzuat.gov.tr `MevzuatMetin/yonetmelik/7.5.4883.pdf`; 6–8. maddeler RG 7.11.2025/33070 ile değişik.

- **7 (2025):** sığınak aranmaz:
  - a) 10'dan az bağımsız bölümlü, yalnız konut kullanımlı yapıda;
  - b) emsal hesabına konu alanı 1500 m²'den az konut dışı yapıda (resmi binada 1000 m²);
  - c) konut bağımsız bölümü 10'dan az ve binanın tamamının emsale konu alanı 1500 m²'den az karma yapıda.
- **8/a (2025):**
  - Kişi başına en az net 1 m².
  - Konutta kişi sayısı bölüm başına: bir odalı 2, iki odalı 3, üç ve daha çok odalı 4.
  - Diğer bina ve tesislerde emsale konu alan ÷ 20.
  - Her 100 kişiye kadın ve erkek için ayrı birer tuvalet ve lavabo; küsurat 50'den fazlaysa birer tane daha.
  - 100 m² üstü sığınakta en az 2,30 m²'lik mutfak nişi. İç yükseklik net 2,40 m'den az olamaz.
- **8/c:** tuvalet, duş ve mutfak nişi hariç net alanı 100 m²'den büyük sığınakta iki çıkış zorunludur.

## Karar

- **Hesap tek yerde** (`kentos_domain::provisions`, `Document`'ı okur, değiştirmez):
  - `parking_report()` ve `shelter_report()` bina programının bağımsız bölümlerini (ortak alan hariç) ve
    yönetmelik brüt alanlarını (`paiy_areas`, ADR 8029) okur.
  - **Bölümün Ek-1 satırı niteliğinden** (`parking_use`): niteliksiz ya da konut niteliğinde bölüm konut; Dükkân,
    Mağaza, Banka, Market dükkân satırı; Büro, Ofis büro satırı. Başka nitelik (otel, okul …) "Ek-1'in bu satırı
    okunmuyor" yazılır, ihtiyacı sıfır sayılmaz, "—" gösterilir.
  - Konut eşikleri: 80'in altı 1/3; 80–120 dahil 1/2; 120'den büyük 180 dahil 1; 180'den büyük 2. Ek-1'in
    "80–120", "120–180" yazımı sınır değerin hangi banda girdiğini söylemez; 120 ve 180 alt banda alındı.
  - Dükkânın, büronun ve konut dışı bölümün sığınak kişisi de **bölümün yönetmelik brütüyle** hesaplanır. Emsale konu
    alan (ADR 8030) bölüm başına değil kat başına tutulur; bölüm brütü ondan büyük ya da eşittir, yani hesap güvenli
    taraftadır.
  - Toplam ihtiyaç bir üste yuvarlanır (8/4); satırlar kesirli yazılır (0,50; 1,20).
- **Çizimdeki karşılık:**
  - Otopark: adı ya da kullanımı Otopark veya Garaj ile başlayan mahallerin net alanı ÷ 20 m², aşağı yuvarlanır
    (4/1-ç; manevra alanı dahil). Park yerleri tek tek sayılmaz; engelli ve bisiklet yerleri bu işte okunmaz.
  - Sığınak: adı Sığınak olan mahallerin net alanı.
  - Konutta oda sayısı: bölümün oturma odası ve yatak odası mahalleri (M33'ün `RoomKind::Living`, `Bedroom`);
    odası çizilmemiş daire bir odalı sayılır.
- **Mahal adları tek yerde:** `RoomKind`, Türkçe sadeleştirme (`fold`, â/î/û dahil), `dwelling_nature`, `stair_name`
  `kentos_domain::room_names`'e taşındı. Mevzuat grubu (`regulation_audit`) ve kaçış yolları (`escape`) aynı adları
  oradan okur.
- **İki canlı tablo** (`TableSource::Parking`, `TableSource::Shelter`; KCAD 227):
  - Metraj ▾ › **Otopark hesabı**, **Sığınak hesabı** (`arch.parkingTable`, `arch.shelterTable`; OTOPARK, SIGINAK).
    Emsal ve TAKS tablosu gibi imleçten yerleştirilir, mahal, program ve ayar değişince aynı adımda yenilenir
    (`reads_settings`, `follow_tables`).
  - Otopark: bölüm, brüt alan, Ek-1 satırı, gereken; "Toplam (kesir bir üste, 8/4)"; "Çizimdeki otopark" (20 m²'ye 1,
    yetmezse kırmızı); 20 yer ve üstünde "Elektrikli araç şarj yeri" (%5, en az 1).
  - Sığınak: bölüm, esas ("2 oda", "60.00 m² ÷ 20"), kişi; "Toplam (kişi başına 1 m², 8/a)"; "Çizimdeki sığınak"
    (yetmezse kırmızı); sığınak aranmıyorsa nedeni (7/a, 7/b, 7/c), aranıyorsa tuvalet-lavabo sayısı ve çıkış sayısı.
  - Tam 50 kişilik küsurat ek tuvalet istemez: yönetmelik "50'den fazla"ya ekler, "50'den az"a eklemez; 50 için
    söylemez, ek istenmeyen taraf seçildi.
- **Denetim › Mevzuat** (`provisions_audit.rs`; kaçış kurallarından sonra):
  - `parking_short`: "Otopark: gereken 7 yer, çizimde 5 (100.00 m² ÷ 20 m²) — Otopark Yön. 8, Ek-1 (16.6.2022)".
    İhtiyaç sıfırsa söylenmez.
  - `shelter_short`: "Sığınak: 33 kişi için en az 33.00 m² net, çizimde 25.00 m² — Sığınak Yön. 8/a (7.11.2025)";
    7/a–c'de söylenmez.
  - `shelter_exit_count`: 100 m²'yi aşan sığınağın kapı ve boşluk sayısı (kaçış grafiğinin geçişleri) 2'den azsa (8/c).
  - `shelter_height_low`: sığınağın kendi yüksekliği, yoksa katının yüksekliği 2,40 m'den azsa (8/a). Kat yüksekliği
    döşemeden döşemeye olduğu için net iç yükseklikten büyüktür; yalnız açık eksik yakalanır.
  - Kaynaklar `Source::Otopark` ("Otopark Yön."), `Source::Siginak` ("Sığınak Yön.").
- Tek evlik konutta da otopark söylenir (Ek-1 daire sayısından bağımsızdır); M33'ün küçük konutu bu yüzden bir bulgu
  daha verir.
- **Yeni ikonlar** `parkingTable` (P levhası ve tablo), `shelterTable` (kalın duvarlı oda ve tablo).

## Ölçüm

- **Test** (`provisions::tests`): 75/80/120/150/181 m² → 1/3, 1/2, 1/2, 1, 2; Daire, Dükkân, Banka şubesi, Büro,
  Otel nitelikleri; 1/2/5 oda → 2/3/4 kişi; 40/151/250 kişi → 0/2/2 tuvalet.
- **Test** (`provisions_tests::the_parking_and_shelter_tables_count_the_units_against_the_drawing`): 10 daire
  (100 m², oturma ve yatak odası), 60 m² dükkân, 100 m² Otopark, 25 m² Sığınak.
  - Otopark tablosu: daire başına 0.50, dükkân 1.20, toplam 7; çizimdeki 100.00 m² → 5, kırmızı.
  - Sığınak tablosu: daire başına "2 oda | 3", dükkân "60.00 m² ÷ 20 | 3", toplam 33 kişi 33.00 m²; çizimdeki 25.00,
    kırmızı; "kadın 0 · erkek 0".
  - Mevzuat iki bulguyu madde ve RG ile söyler.
  - 50 m²'lik "Otopark 2" eklenince tablo aynı adımda 150.00 m² → 7 yazar.
- **Test** (`provisions_tests::a_building_of_fewer_than_ten_flats_asks_no_shelter`): 9 daire → 7/a, bulgu yok.
- **Test** (M33'ün küçük konutu): 1 gereken yer, 0 çizilmiş; Büro olunca koridor ve otopark iki ileti.
- **Gerçek pencere** (1920 × 1080; `fixtures/interaction/real/otopark-siginak.txt`, testin sakladığı çizim):
  - Metraj ▾ › Otopark hesabı imleçten yerleşti; Sığınak hesabı Menülerde ara "sığınak hesabı" ile.
  - Önce (M35 ikilisi): Mevzuat 10 satır (yalnız PAİY 29/1), toplam 12 bulgu. Sonra: 12 satır, 14 bulgu; 11. ve 12.
    satır otopark ve sığınak.
  - Resimler: `.run/gui-evidence/mimari-m36/m36-tablolar.png`, `m36-mevzuat.png`, `m36-simgeler.png`,
    `m36-kaynaklar.png`.
- **KCAD:** `provision-tables.kcad` (iki tablo); kırık `provision-tables-in-schema-226`; 246 geçerli dosya, 1713 dosya
  Python okuyucusuyla tutarlı.

## Sonraki adımlar

- Ek-1 tablosunun Resmî Gazete ekinden okunup konut ve büro değerlerinin onunla karşılaştırılması.
- Park yerlerinin tek tek çizimi ve sayımı (2,50 × 5,00 m, rampa, engelli yeri), otopark katının manevra denetimi.
- Ek-1'in öbür satırları (konaklama, sağlık, eğitim, AVM …) ve belediye otopark yönetmeliği katmanı (8/2).
- Emsale konu alanın bölüm bölüm dağıtımı; resmi bina (7/b, 1000 m²) seçeneği.
- Sığınağın duvar kalınlığı (8/b, 60 cm beton), demir kapı, havalandırma ve mutfak nişi denetimi.
