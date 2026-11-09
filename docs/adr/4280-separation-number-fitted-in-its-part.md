# 4280 — Ayırma çapında dış parçanın numarası parçaya sığar

Tarih: 2026-11-09. plan/map-cad.md MAP-11, sekizinci tur. Durum: taslak. ADR 3480'in ve 3880'in “Sonra”sındaki
“numaranın yazısının parçanın içine sığdırılması (dar parçada etiket parçanın dışına taşıyor)”.

İlgili:
- [ADR 3480](3480-separation-sheet-piece-numbers-drawn.md): dış parçanın numarası çizimde (etiketli nokta, `label_at`);
- [ADR 3080](3080-separation-sheet-outside-part-numbers.md): dış parçaların parsel numaraları (m. 13/3);
- [ADR 0145](0145-text-extras.md): yazının kutusu ve hizası.

## Dayanak

- **Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik** (ADR 3080'in ve 3480'in okuması): m. 13(3), dışarıda kalan her
  parça parsel numarası verilerek tescil edilir. Yönetmelik numaranın çapta nereye ve hangi boyda yazılacağını
  söylemez.
- **KentOS bugün** (ADR 3480):
  - numara, parçanın orta yüksekliğindeki yatay çizginin en geniş aralığının ortasında etiketli bir noktadır;
  - nokta etiketi dönmez, boyu katmanın etiket biçemindendir;
  - 4 m genişliğindeki parçada (101/10'un dış parçaları) “101/24” parçanın dışına taşıyordu.
- **Yazının kutusu** (ADR 0145): tek satırlık yazı, harflerinin ilerlemeleri (çizimin yazı tipi) × boyu genişliktedir.
  Taban çizgisinin 0,23 boy altından 1,15 boy üstüne uzanır, dönüşüyle döner.

## Karar

1. **`cad.readjustment.sheets`'e `numberHeight`** (kâğıtta mm, kullanıcının; KentOS boy vermez):
   - Verilince her dış parçanın numarası nokta yerine bir **yazı** olur: parçanın katmanında, `Parsel`, `Kısım`,
     `ParselNo` öznitelikleriyle, ortası `at`'ta.
   - Verilmezse ADR 3480'deki gibi etiketli nokta kalır; eski durumlar değişmedi.
2. **Sığdırma** (`fitted`, belirli kural; bağımsız başvuru aynısını bulur):
   - **Boy:** `numberHeight` × çapın ölçek paydası / 1000 metre; sığmazsa onda bir adımlarla (1, 0,9 … 0,1) küçülür.
   - **Dönüş:** önce yatay; sonra parçanın dış halkasının en uzun düz kenarı boyunca (eşitlerden ilki, soldan okunur:
     (−90°, 90°]), yataydan başkaysa.
   - **Yer:** her dönüşte `label_at`'ın kuralı, parça o dönüşle çevrilmiş olarak (orta çizginin en geniş aralığının
     ortası, geri çevrilir).
   - **İçeride:** yazının kutusu (ADR 0145, çizimin yazı tipiyle) parçanın içindeyse, yani parçanın kutuyla kesişimi
     kutunun alanını 10⁻⁹ payla veriyorsa.
   - **Seçim:** en büyük pay kazanır, eşitlerde yatay. Hiçbiri sığmazsa yatay, en küçük payda (0,1), `fits` false.
3. **Cevap:** her çapın `labels`'ında, `numberHeight` ile `height` (m), `rotation` (derece), `share` ve `fits`; `at`
   yazının ortasıdır.
4. **Ret:** `numberHeight` 0'dan büyük değilse `invalid_input` (`numberHeight`).
5. **Masaüstü** (Düzenleme penceresinin Ayırma çapı görünümü): Dış parça no'nun altında **Numara yazısı (mm)** kutusu;
   açıklaması “parçaya sığana dek onda bir adımlarla küçülür; yatay ya da en uzun kenarı boyunca”. Boş: etiketli
   nokta. Okunmayan değer söylenir, hiçbir şey yazılmaz.
6. **Kapsam işareti:** ADR 2680'deki gibi **Harita ve CBS**. Çap taslaktır; yazının boyu ve yeri resmî bir kural değil,
   KentOS'un sığdırma kuralıdır.

## Sınırlar

- Yaylı dış parçada en uzun kenar düz kenarlar arasından seçilir; yayın teğeti boyunca dönüş yok.
- Sığmayan numara için parçanın dışında kılavuz çizgili yazı yok; `fits` false söylenir.
- Numaranın boyu için en küçük okunur boy kuralı yok (resmî değer okunmadı); kullanıcı payı görür.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/readjustment_sheets_cases.py`; KentOS kodu yok), 5 yeni durum (43):
  - yazı tipinin ilerlemeleri yazı kütüphanesi olmadan dosyadan okunur (head, hhea, hmtx, cmap biçim 4);
  - parça kutuyla Sutherland–Hodgman'la kırpılır;
  - durumlar: 2 mm'de her parçaya tam boy (101/10'un 4 m'lik parçalarında 90° dönük); 6 mm'de 0,4'e küçülme; 400
    mm'de hiçbiri sığmaz; plan; boy 0 reddi. Eski durumlar değişmedi.

  Rust ve Python geçer.
- **Python** (`python/tests/test_separation_number_fit.py`), testin kendi okumasıyla (fontTools, kutunun köşeleri
  parçada):
  - 2 mm'de yazılar 90° ve tam boyda, kutuları parçada;
  - 6 mm'de payı bir onda bir büyük olsa sığmaz;
  - boy verilmeyince etiketli noktalar.

  ADR 3480'in Python testi değişmeden geçer.
- **MCP** (`crates/native/mcp/tests/separation_number_fit.rs`): etiketler durumunki, her numara etiketindeki boy ve
  dönüşle bir yazı, etiketli nokta yok. MCP'nin ana testleri geçer.
- **Masaüstü** (`readjustment_window_tests::the_numbers_typed_with_a_height_are_texts_fitted_in_their_parts`):
  - okunmayan boy söylenir;
  - 2 mm'de 101/10'un parça katmanında iki yazı, 2 m boyunda, 90° dönük;
  - nokta yok.

  Düzenleme penceresinin öbür testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/numara-sigdir-4280/`
  (pencere, 101/10'un paftası, modelde dar parçanın numarası).

## Sonra

- Sığmayan numaranın parçanın dışına kılavuz çizgiyle yazılması; yaylı parçada yay boyunca.
- Tescil bildirimi (m. 22(1)(ğ)): TKGM'nin biçimi beklenir; değer esaslı dağıtım (resmî metinde kural yok).
- Web (TODOS §0.2).
