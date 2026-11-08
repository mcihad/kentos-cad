# 2280 — Ayırma çapının parçaları, sınırları ve koordinatları

Tarih: 2026-11-08. plan/map-cad.md MAP-11, üçüncü tur. Durum: taslak. Plan satırı: “Arazi düzenlemesi … hesap,
tutanak ve çıktıları denetlenebilir olsun.”

İlgili: [ADR 2060](2060-readjustment-scope-and-separation-sheet.md), düzenleme kapsamı ve ayırma çapının sayıları. Bu ADR
onun “Sonra”sındaki “ayırma çapının çizimi: parçaların sınırları ve koordinatları” maddesini kapatır. Çıktının paftası
sonraya kalır.

## Dayanak

- **Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik**, RG 22.02.2020/31047. ADR 2060'ın okuduğu metin; Resmî
  Gazete'nin PDF'i, TÜRMOB'un kopyası, 7 Kasım 2026:
  - **m. 4(c):** ayırma çapı, bir kısmı düzenleme sahasında kalan parsel için düzenlenen krokidir;
  - **m. 13(2):** çapta tescilli ve hesaplanan yüzölçümü, fark ve yanılma sınırı, giren ve girmeyen kısımların
    yüzölçümü, **sınırları ve koordinatları** gösterilir;
  - **m. 13(3):** dışarıda kalan parçaların her biri ayrı parsel numarası alır.
- **KentOS bugün** (ADR 2060):
  - Kapsam, çapın sayılarını veriyordu: tescilli, hesaplanan, fark, sınır, giren, girmeyen;
  - parçaları çekirdeğin kesin alan cebiriyle buluyordu, ama sınırlarını ve koordinatlarını dışarı vermiyordu;
  - m. 13(2)'nin son kısmı eksikti.

## Karar

1. **Cevapta `parts`.** `cad.readjustment.scope`, kısmen giren her parselin parçalarını verir: tarafı (`inside`,
   `outside`), alanı ve halkaları.
   - **Hangi parçalar:** toleransın saydığı parçalar (ortalama genişlik 2A/P > tolerans, ADR 2060'taki gibi).
   - **Sıra:** önce içeridekiler, sonra dışarıdakiler. Her tarafın parçaları ilk köşelerine göre dizilir (doğu, sonra
     kuzey).
   - **Halkaların kanonik biçimi:** başvuru ile KentOS aynı köşeleri aynı sırada yazsın diye:
     - dış halka saat yönünün tersine, delikler saat yönünde, deliklerin sırası ilk köşelerine göre;
     - her halka en batıdaki, eşitlikte en güneydeki köşesinden başlar;
     - iki doğru kenarın aynı yönde sürdüğü köşe yazılmaz (kesin yön testi);
     - yay kenarı kabarıklığıyla kalır (`bulges`; yoksa hepsi doğru).
   - Tamamı giren ve dışarıdaki parselde `parts` yoktur. Ek bir alandır: komut sürümü değişmez.
2. **Masaüstü** (Düzenleme dağıtım taslağı):
   - Yeni görünüm **Ayırma çapı**: kısmen giren her parselin her parçasının köşeleri satır satır. Sütunlar Parsel,
     Kısım, Köşe, Y, X ve Alan (m²); alan parçanın ilk satırında.
   - Kısım adları:
     - Giren;
     - Girmeyen, birden çok dış parça varsa Girmeyen 1, 2 … (m. 13(3): her biri ayrı parsel);
     - deliğin köşeleri “(delik)” diye.
   - **Çapı çizime yaz** (bu görünümde): parçaları “Ayırma çapı” katmanına alan olarak yazar. Katman yoksa oluşturulur.
     Öznitelikler Parsel, Kısım ve Alan (m²)'dır.
   - Yazım tek geri alma adımıdır: “Ayırma çapı”, katman da dahil. Yazan `cad.entities.create`'tir; Python ve MCP aynı
     parçaları `parts`'tan alıp aynı komutla yazabilir.
3. **Hesaplanan alan tescilli alanın üstüne yazılmaz** (ADR 2060 §2). Çap taslaktır, resmî kroki değildir.
4. **Kapsam işareti:** ADR 2060'taki gibi **Harita + CBS**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/readjustment_scope_cases.py`, KentOS kodu yok.
  - Parçalar kesin kesirlerle hesaplanır: içeride Sutherland–Hodgman kırpması.
  - Dışarıdaki parça hücrelerinin birleşimidir: her hücre saat yönünün tersine çevrilir, kenarları öbür köşelerde
    bölünür, iki yönde de geçen parçalar atılır, kalanlar halkaya bağlanır. Sonra kanonik biçim uygulanır.
  - 33 durum (31'den). Kısmen giren parsellerin cevapları `parts`'ı taşır. Yeni iki durum:
    - köşesi sınırın dışında kalan parsel: girmeyen kısım iki hücreden L biçiminde tek parça (6 köşe);
    - sınır parselin içinde: girmeyen kısım delikli tek parça (dış halka ve saat yönünde delik).
  - Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/separation_sheet.rs`): üç durumun çizimi `.kcad` olarak açılır. Parçalar
  durumunkilerle 10⁻⁶ içinde aynıdır: L, delikli halka ve iki dış parça.
- **Masaüstü** (`readjustment_window_tests.rs`, `the_separation_sheet_lists_the_parts_and_writes_them`):
  - satırlar durumun köşeleridir; dokuz parça vardır, 101/10'un parçaları Giren, Girmeyen 1, Girmeyen 2'dir;
  - Çapı çizime yaz dokuz alanı öznitelikleriyle yeni katmana yazar;
  - tek geri alma parçaları ve katmanı kaldırır.

  Çapraz masaüstü denetimleri 137 geçer. Resimler `.run/gui-evidence/ayirma-capi-2280/`: çapın köşeleri ve çizimde
  parçalar.

## Sonra

- Ayırma çapının paftası: parsel, sınır, parçalar, alan ve koordinat tablosu tek sayfada (pafta modülüyle). Yapıldı:
  [ADR 2680](2680-separation-sheets.md).
- m. 8(5)'in geçici koordinatlı parsel yanılma sınırı (tablosu resim olarak duruyor; ADR 2060).
- Birden çok dış parça arasında hatanın paylaşımı (yönetmelik kural vermiyor; KentOS uydurmaz).
